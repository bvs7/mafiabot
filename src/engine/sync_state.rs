use core::time;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Condvar, Mutex, MutexGuard, TryLockError},
    thread::{park_timeout, JoinHandle, Thread},
    time::Duration,
};

use chrono::{DateTime, Local};
use night_action::{Act, NightAct};
use serde::{Deserialize, Serialize};
use toml::value::Date;
use tracing::error;

use super::{
    id::{Ballot, Choice, Gid, Pid, RawBallot, RawChoice},
    phase::{Blocks, Phase, PhaseKind, Votes},
    players::{Cause, Context, PlayerState, Players},
    Action, Error, Event, EventTx, Role, RoleKind, Rules, Team,
};

const ELECTION_DELAY: Duration = Duration::from_secs(10);
const DAWN_DELAY: Duration = Duration::from_secs(10);
const TICK_DELAY: Duration = Duration::from_secs(1);
/*
Have scheduled events? Or at least a thread that will notify when needed.
*/

mod async_game {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use chrono::{DateTime, Local, OutOfRangeError};
    use tokio::{
        sync::{Mutex, RwLock, TryLockError},
        time::timeout,
    };

    use crate::engine::{
        id::Pid, Action, ActionMsg, ActionResponder, ActionRx, ActionTx, EventRx, EventTx,
    };

    use super::{State, Status};

    enum NoAction {
        Timeout,
        Closed,
    }

    impl From<OutOfRangeError> for NoAction {
        fn from(_: OutOfRangeError) -> Self {
            NoAction::Timeout
        }
    }
    impl From<tokio::time::error::Elapsed> for NoAction {
        fn from(_: tokio::time::error::Elapsed) -> Self {
            NoAction::Timeout
        }
    }

    struct Game {
        state: RwLock<State>,
        action_rx: Mutex<ActionRx>,
        action_tx: ActionTx,
        event_tx: EventTx,
    }

    impl Game {
        pub fn new(mut state: State) -> Self {
            let (action_tx, action_rx) = tokio::sync::mpsc::channel(100);
            let (event_tx, _) = tokio::sync::broadcast::channel(100);
            state.event_tx = Some(event_tx.clone());
            Self {
                state: RwLock::new(state),
                action_rx: Mutex::new(action_rx),
                action_tx,
                event_tx,
            }
        }

        pub fn action_tx(&self) -> ActionTx {
            self.action_tx.clone()
        }

        pub fn event_rx(&self) -> EventRx {
            self.event_tx.subscribe()
        }

        pub async fn status(&self, names: HashMap<impl Into<Pid>, String>) -> Status {
            let state = self.state.read().await;
            state.status(names)
        }

        pub fn start(self) -> Arc<Self> {
            let game = Arc::new(self);
            let g = game.clone();
            tokio::spawn(async move { g.run().await });
            game
        }

        /// Run the game loop, which involves waiting for actions or for timers to expire,
        /// handling the actions, and updating the game state.
        async fn run(&self) -> Result<(), TryLockError> {
            let mut action_rx = self.action_rx.try_lock()?;
            let mut alarm_time = None;
            loop {
                match self.next_action(alarm_time, &mut action_rx).await {
                    Ok((action, resp)) => {
                        self.action(action, resp).await;
                    }
                    Err(NoAction::Timeout) => {}
                    Err(NoAction::Closed) => break,
                }
                alarm_time = self.update().await;
            }
            Ok(())
        }

        async fn next_action(
            &self,
            alarm_time: Option<DateTime<Local>>,
            action_rx: &mut ActionRx,
        ) -> Result<ActionMsg, NoAction> {
            let dur = match alarm_time {
                Some(time) => (time - Local::now()).to_std()?,
                None => Duration::MAX,
            };
            let action = timeout(dur, action_rx.recv()).await?;
            match action {
                Some((action, resp)) => Ok((action, resp)),
                None => Err(NoAction::Closed),
            }
        }

        /// First check action with read lock. If action is invalid, return error.
        /// Then check action again with write lock, to be sure it is still valid.
        /// If action is still valid, send ok then perform action.
        async fn action(&self, action: Action, resp: ActionResponder) {
            let rstate = self.state.read().await;
            if let Err(err) = rstate.validate_action(&action) {
                let _ = resp.send(Err(err));
                return;
            }
            drop(rstate);
            let mut wstate = self.state.write().await;
            let action = match wstate.validate_action(&action) {
                Ok(action) => action,
                Err(err) => {
                    let _ = resp.send(Err(err));
                    return;
                }
            };
            resp.send(Ok(()));
            wstate.perform_action(action);
        }

        async fn update(&self) -> Option<DateTime<Local>> {
            let mut wstate = self.state.write().await;
            wstate.update()
        }
    }
}

fn count_votes(votes: &Votes, players: &Players) -> HashMap<Choice, Vec<Pid>> {
    let mut vote_list: HashMap<Choice, Vec<Pid>> = HashMap::new();
    for (pid, _) in players.alive() {
        vote_list.insert(Some(pid), vec![]);
    }
    vote_list.insert(None, vec![]);
    for (voter, choice) in votes {
        vote_list.entry(*choice).or_default().push(*voter);
    }
    vote_list
}

fn thresh(n: usize, choice: &Choice) -> usize {
    if choice.is_some() {
        return (n / 2) + 1;
    } else {
        return (n + 1) / 2;
    }
}

fn last_vote_for(event_log: &Vec<Event>, choice: &Choice) -> Option<Pid> {
    for event in event_log.iter().rev() {
        match event {
            Event::Vote { voter, ballot: Some((c, _)), .. } => {
                if c == choice {
                    return Some(*voter);
                }
            }
            _ => {}
        }
    }
    None
}

struct Status {
    day: u32,
    phase: PhaseKind,
    votes: Option<HashMap<Choice, Vec<Pid>>>,
    count: HashMap<Team, usize>,
    players: Vec<Pid>,
    rules: Rules,
    names: HashMap<Pid, String>,
}

impl Status {
    pub fn brief(&self) -> String {
        let mut msg = String::new();
        msg.push_str(&format!("{} {}:\n  ", self.phase, self.day));
        msg.push_str("  Teams:\n");
        if let Some(town) = self.count.get(&Team::Town) {
            msg.push_str(&format!("    Town: {}\n", town));
        }
        if let Some(mafia) = self.count.get(&Team::Mafia) {
            msg.push_str(&format!("    Mafia: {}\n", mafia));
        }
        if let Some(rogue) = self.count.get(&Team::Rogue) {
            msg.push_str(&format!("    Rogue: {}\n", rogue));
        }
        msg
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let empty = "???".to_string();
        write!(f, "{} {}:\n", self.phase, self.day)?;

        // Display votes
        if let Some(vote_list) = &self.votes {
            let th = thresh(self.players.len(), &Some(Pid::from(0)));
            let pth = thresh(self.players.len(), &None);
            write!(f, "  Votes:\n")?;
            for pid in self.players.iter() {
                let votes = vote_list.get(&Some(*pid));
                if let Some(votes) = votes {
                    let name = self.names.get(pid).unwrap_or(&empty).to_string();
                    write!(f, "    {}({}/{}): ", name, votes.len(), th)?;
                    let names = votes
                        .into_iter()
                        .map(|pid| self.names.get(pid).unwrap_or(&empty).to_string())
                        .collect::<Vec<_>>();
                    let names_str = names.join(", ");
                    write!(f, "{names_str}\n")?;
                }
            }
            let votes = vote_list.get(&None);
            if let Some(votes) = votes {
                write!(f, "    Abstain({}/{}): ", votes.len(), pth)?;
                let names = votes
                    .into_iter()
                    .map(|pid| self.names.get(pid).unwrap_or(&empty).to_string())
                    .collect::<Vec<_>>();
                let names_str = names.join(", ");
                write!(f, "{names_str}\n")?;
            }
        }
        write!(f, "  Teams:\n")?;
        if let Some(town) = self.count.get(&Team::Town) {
            write!(f, "    Town: {}\n", town)?;
        }
        if let Some(mafia) = self.count.get(&Team::Mafia) {
            write!(f, "    Mafia: {}\n", mafia)?;
        }
        if let Some(rogue) = self.count.get(&Team::Rogue) {
            write!(f, "    Rogue: {}\n", rogue)?;
        }
        Ok(())
    }
}

enum ValidAction {
    Vote(Pid, Ballot),
    Reveal(Pid),
    Target(Pid, Choice),
    Scheme(Pid, Choice),
    EclipseVote(Pid),
}

// TODO: have event log be an generic trait, so that an implementation can define it?
#[derive(Debug, Clone)]
pub struct State {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    event_tx: Option<EventTx>,
}

impl State {
    pub fn new(registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>, rules: Rules) -> Self {
        Self {
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_tx: None,
        }
    }

    pub fn get_target(&self, idx: usize) -> Result<Choice, Error> {
        self.players.get_target(idx)
    }

    pub fn tx(&mut self, event: Event) {
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(event);
        }
    }

    pub fn status(&self, names: HashMap<impl Into<Pid>, String>) -> Status {
        let names = names.into_iter().map(|(k, v)| (k.into(), v)).collect();
        Status {
            day: self.day,
            phase: self.phase.kind(),
            votes: self.phase.vote_list().ok(),
            count: self.players.counts(Team::from),
            players: self.players.list(),
            rules: self.rules.clone(),
            names: names,
        }
    }

    fn start(&mut self) {
        self.tx(Event::Start { players: self.players.alive(), rules: self.rules.clone() });
        let n = self.players.n();
        if n % 2 == 1 {
            self.day(HashMap::new());
        } else {
            self.night();
        }
    }

    pub fn validate_action(&self, action: &Action) -> Result<ValidAction, Error> {
        use Action::*;
        match action {
            Vote { voter, ballot } => self.validate_vote(*voter, *ballot),
            Target { actor, choice } => self.validate_target(*actor, *choice),
            Scheme { killer, mark } => self.validate_scheme(*killer, *mark),
            Reveal { actor } => self.validate_reveal(*actor),
        }
    }

    pub fn perform_action(&mut self, action: ValidAction) {
        use ValidAction::*;
        match action {
            Vote(voter, ballot) => self.vote(voter, ballot),
            Target(actor, choice) => self.target(actor, choice),
            Scheme(killer, mark) => self.scheme(killer, mark),
            Reveal(celeb) => self.reveal(celeb),
            EclipseVote(victim) => self.eclipse_vote(victim),
        }
    }

    fn validate_reveal(&self, celeb: u64) -> Result<ValidAction, Error> {
        let celeb = self.players.validate(celeb)?;
        let role = self.players.get(celeb);
        if role != Role::CELEB {
            return Err(Error::ExpectedCeleb { actual: role.kind() });
        }
        if !matches!(self.phase, Phase::Day { .. }) {
            return self.phase.expected(PhaseKind::Day);
        }
        Ok(ValidAction::Reveal(celeb))
    }

    pub fn reveal(&mut self, celeb: Pid) {
        let Phase::Day { blocks, .. } = &self.phase else {
            return panic!("Expected Day phase");
        };
        if let Some(blockers) = blocks.get(&celeb) {
            self.tx(Event::Block { blocked: celeb, blockers: blockers.clone() });
        } else {
            self.tx(Event::Reveal { celeb });
        }
    }

    fn validate_vote(&self, voter: u64, ballot: RawBallot) -> Result<ValidAction, Error> {
        let voter = self.players.validate(voter)?;
        let ballot = self.players.validate_ballot(ballot)?;
        let Phase::Day { votes, .. } = &self.phase else {
            if matches!(self.phase, Phase::Eclipse { .. }) {
                return self.validate_eclipse_vote(voter, ballot);
            }
            return self.phase.expected(PhaseKind::Day);
        };
        if votes.get(&voter) == ballot.as_ref() {
            return Err(Error::IneffectiveAction);
        }
        Ok(ValidAction::Vote(voter, ballot))
    }

    fn vote(&mut self, voter: Pid, ballot: Ballot) {
        let Phase::Day { votes, .. } = &mut self.phase else {
            panic!("Expected Day phase");
        };
        let former = if let Some(choice) = ballot {
            votes.insert(voter, choice)
        } else {
            votes.remove(&voter)
        };
        let vote_list = count_votes(votes, &self.players);

        let ballot_check = ballot.clone().map(|c| (c, vote_list.get(&c).unwrap().len()));
        let former_check = former.clone().map(|c| (c, vote_list.get(&c).unwrap().len()));

        self.tx(Event::Vote { voter: voter.clone(), ballot: ballot_check, former: former_check });
        self.check_election(voter, ballot, vote_list);
    }

    fn check_election(&mut self, voter: Pid, ballot: Ballot, vote_list: HashMap<Choice, Vec<Pid>>) {
        let Phase::Day { elect, .. } = &mut self.phase else {
            panic!("Expected Day phase");
        };

        let n = self.players.n();
        if let Some((choice, hammer, time)) = elect {
            if vote_list.get(&choice).unwrap().len() < thresh(n, &choice) {
                // Election averted
                *elect = None;
            }
        }
        if let Some(choice) = ballot {
            let voters = vote_list.get(&choice).unwrap();
            if voters.len() >= thresh(n, &choice) {
                let hammer = voter;
                let time = Local::now() + ELECTION_DELAY;
                *elect = Some((choice, hammer, time));
            }
        }
    }

    // TODO: check stripper
    fn validate_target(&self, actor: u64, choice: RawChoice) -> Result<ValidAction, Error> {
        let actor = self.players.validate(actor)?;
        let choice = self.players.validate_choice(choice)?;
        let role = self.players.get(actor);
        if !role.is_targeting() {
            return Err(Error::ExpectedTargetingRole { actual: role.kind() });
        }
        let Phase::Night { targets, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Night);
        };
        if targets.get(&actor).is_some() {
            return Err(Error::IneffectiveAction);
        }
        Ok(ValidAction::Target(actor, choice))
    }

    pub fn target(&mut self, actor: Pid, choice: Choice) {
        let Phase::Night { targets, .. } = &mut self.phase else {
            panic!("Expected Night phase");
        };
        targets.insert(actor, choice);
        self.tx(Event::Target { actor, choice });
        self.check_dawn();
    }

    fn validate_scheme(&self, killer: u64, mark: RawChoice) -> Result<ValidAction, Error> {
        let killer = self.players.validate(killer)?;
        let mark = self.players.validate_choice(mark)?;
        let role = self.players.get(killer);
        if !role.is_scheming() && mark.is_some() {
            return Err(Error::ExpectedSchemingRole { actual: role.kind() });
        }
        let Phase::Night { scheme, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Night);
        };
        if scheme.is_some() {
            return Err(Error::IneffectiveAction);
        }
        Ok(ValidAction::Scheme(killer, mark))
    }

    pub fn scheme(&mut self, killer: Pid, mark: Choice) {
        let Phase::Night { scheme, .. } = &mut self.phase else {
            panic!("Expected Night phase");
        };
        *scheme = Some((killer, mark));
        self.tx(Event::Scheme { killer, mark });
        self.check_dawn();
    }

    pub fn check_dawn(&mut self) {
        let Phase::Night { targets, scheme, dawn } = &mut self.phase else {
            panic!("Expected Night phase");
        };
        if dawn.is_some() {
            return;
        }
        let mut ready = true;
        if scheme.is_none() {
            ready = false;
        }
        for (pid, role) in self.players.alive() {
            if role.is_targeting() && targets.get(&pid).is_none() {
                ready = false;
            }
        }
        if ready {
            let time = Local::now() + DAWN_DELAY;
            *dawn = Some(time);
        }
    }

    fn dawn(&mut self) -> Blocks {
        self.tx(Event::Dawn);
        let night_actions = NightAct::from_state(self);

        self.apply_night_actions(night_actions)
    }

    fn eclipse(&mut self, avenger: Pid, hammer: Pid, guilty: Vec<Pid>) {
        self.phase = Phase::Eclipse { avenger, hammer, guilty: guilty.clone() };
        self.tx(Event::Eclipse { avenger, hammer, guilty });
    }

    fn validate_eclipse_vote(&self, voter: Pid, ballot: Ballot) -> Result<ValidAction, Error> {
        let Phase::Eclipse { avenger, guilty, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Eclipse);
        };
        if voter != *avenger {
            return Err(Error::IneffectiveAction);
        }
        match ballot {
            Some(Some(victim)) => {
                if victim == *avenger {
                    return Err(Error::IneffectiveAction);
                }
                if !guilty.contains(&victim) {
                    return Err(Error::IneffectiveAction);
                }
                return Ok(ValidAction::EclipseVote(victim));
            }
            Some(None) => return Err(Error::IneffectiveAction),
            None => return Err(Error::IneffectiveAction),
        }
    }

    fn eclipse_vote(&mut self, victim: Pid) {
        let Phase::Eclipse { avenger, hammer, .. } = &self.phase else {
            panic!("Expected Eclipse phase");
        };
        let avenger = *avenger;
        let hammer = *hammer;
        self.vengeance(avenger, victim, hammer);
    }

    fn vengeance(&mut self, avenger: Pid, victim: Pid, hammer: Pid) {
        self.tx(Event::Vengeance { avenger, victim });
        self.eliminate(victim, avenger, Context::new(self.day, Cause::Vengeance));
        self.eliminate(avenger, hammer, Context::new(self.day, Cause::Election));
        if !self.check_end() {
            self.night();
        }
    }

    fn eliminate(&mut self, player: Pid, _culpable: Pid, context: Context) {
        let PlayerState::Alive(role) = self.players.eliminate(&player, context) else {
            panic!("Eliminating a dead player?");
        };
        let role = role.kind();
        self.tx(Event::Eliminate { player, role, context });
    }

    fn day(&mut self, blocks: Blocks) {
        self.day += 1;
        self.phase = Phase::Day { votes: HashMap::new(), blocks, elect: None };
        let counts = self.players.counts(Team::from);
        self.tx(Event::Day { day: self.day, counts });
    }

    fn night(&mut self) {
        self.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };
        let counts = self.players.counts(Team::from);
        self.tx(Event::Night { day: self.day, counts });
    }

    fn check_end(&mut self) -> bool {
        let counts = self.players.counts(Team::from);
        let n = self.players.alive().len();
        let n_maf = counts.get(&Team::Mafia).copied().unwrap_or(0);
        if n_maf == 0 {
            self.phase = Phase::End { winner: Team::Town };
            return true;
        } else if n - n_maf <= n_maf {
            self.phase = Phase::End { winner: Team::Mafia };
            return true;
        }
        false
    }

    fn update(&mut self) -> Option<DateTime<Local>> {
        match &self.phase {
            Phase::Day { elect: Some((choice, hammer, time)), votes, .. } => {
                if time < &Local::now() {
                    // ELECTION
                    let voters = count_votes(votes, &self.players).get(choice).unwrap().clone();
                    let (choice, hammer) = (*choice, *hammer);
                    self.tx(Event::Election { choice, hammer, voters: voters.clone() });
                    if let Some(pid) = choice {
                        // Check for IDIOT
                        if self.players.get(pid) == Role::IDIOT {
                            self.eclipse(pid, hammer, voters);
                            return None;
                        } else {
                            self.eliminate(pid, hammer, Context::new(self.day, Cause::Election));
                        }
                    }
                    if !self.check_end() {
                        self.night();
                    }
                    return None;
                } else {
                    return Some(*time);
                }
            }
            Phase::Night { dawn: Some(time), .. } => {
                if time < &Local::now() {
                    let blocks = self.dawn();
                    if !self.check_end() {
                        self.day(blocks);
                    }
                    return None;
                } else {
                    return Some(*time);
                }
            }
            _ => return None,
        }
    }
}

pub mod night_action {
    use std::collections::HashMap;

    use rand::seq::SliceRandom;

    use crate::engine::{sync_state::State, Event};

    use super::*;

    #[derive(Debug, Clone)]
    pub enum Act {
        Block,
        Save {
            /// Effective if Save targeted the same player as a Kill
            effective: bool,
        },
        Kill {
            saviors: Vec<Pid>,
        },
        Investigate,
        Milk,
    }

    impl Act {
        fn priority(&self) -> i32 {
            use Act::*;
            match self {
                Block => 2,
                Save { .. } => 1,
                Kill { .. } => 0,
                Investigate => -1,
                Milk => -2,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct NightAct {
        pub act: Act,
        pub actor: Pid,
        pub target: Pid,
        pub blockers: Vec<Pid>,
    }

    impl NightAct {
        // Assume role is a targeting role...
        pub fn from_target(role: Role, actor: Pid, target: Pid) -> Self {
            tracing::debug!("Night action from target: {:?}, {:?}, {:?}", role, actor, target);
            use Role::*;
            match role {
                STRIPPER => NightAct { act: Act::Block, actor, target, blockers: vec![] },
                DOCTOR => NightAct {
                    act: Act::Save { effective: false },
                    actor,
                    target,
                    blockers: vec![],
                },
                COP => NightAct { act: Act::Investigate, actor, target, blockers: vec![] },
                MILKY => NightAct { act: Act::Milk, actor, target, blockers: vec![] },
                TOWN | CELEB | MILLER | MAFIA | GOON | GODFATHER | IDIOT | SURVIVOR | AGENT(_)
                | GUARD(_) => panic!("Expected a targeting role"),
            }
        }

        pub fn block(&mut self, blocker: Pid) {
            tracing::debug!("{:?} blocking {:?}", blocker, self);
            self.blockers.push(blocker);
        }

        pub fn save(&mut self, savior: Pid) {
            tracing::debug!("{:?} saving from {:?}", savior, self);
            match &mut self.act {
                Act::Kill { saviors, .. } => saviors.push(savior),
                _ => {}
            }
        }

        pub fn from_state(state: &State) -> Vec<Self> {
            let players = &state.players;
            let Phase::Night { targets, scheme, .. } = &state.phase else {
                panic!("To night actions during not night");
            };
            let mut night_actions: Vec<NightAct> = targets
                .into_iter()
                .flat_map(|(a, t)| t.map(|t| NightAct::from_target(players.get(*a), *a, t)))
                .collect();
            if let Some((killer, Some(target))) = scheme {
                night_actions.push(NightAct {
                    act: Act::Kill { saviors: vec![] },
                    actor: *killer,
                    target: *target,
                    blockers: vec![],
                })
            };

            // Compare enums, prioritized by order of NightAction
            // (shuffle before to ensure no ordering to things like milking)
            night_actions.shuffle(&mut rand::thread_rng());
            night_actions.sort_by(|a, b| b.act.priority().cmp(&a.act.priority()));

            for i in 0..night_actions.len() {
                let (earlier, rest) = night_actions.split_at_mut(i);
                let cur = &mut rest[0];
                for pre in earlier.iter_mut() {
                    if cur.act.priority() >= pre.act.priority() {
                        // Acts with the same priority do not effect each other
                        continue;
                    }
                    match &mut pre.act {
                        Act::Block if pre.target == cur.actor => cur.block(pre.actor),
                        Act::Save { effective } if pre.target == cur.target => {
                            if matches!(cur.act, Act::Kill { .. }) {
                                *effective = true;
                            }
                            if pre.blockers.is_empty() {
                                cur.save(pre.actor);
                            }
                        }
                        _ => {}
                    }
                }
            }
            return night_actions;
        }
    }

    impl State {
        pub fn apply_night_actions(&mut self, night_actions: Vec<NightAct>) -> Blocks {
            let mut blocks: HashMap<Pid, Vec<Pid>> = HashMap::new();
            let mut kills: HashMap<Pid, Pid> = HashMap::new();
            for na in night_actions {
                match na.act {
                    Act::Block => {
                        blocks.entry(na.target).or_default().push(na.actor);
                    }
                    Act::Save { effective } => {
                        if effective && !na.blockers.is_empty() {
                            self.tx(Event::Block {
                                blocked: na.actor,
                                blockers: na.blockers.clone(),
                            });
                        }
                    }
                    Act::Kill { saviors } => {
                        if saviors.is_empty() {
                            let killer = na.actor;
                            let mark = na.target;
                            kills.insert(mark, killer);
                        } else {
                            self.tx(Event::Save { saved: na.target, saviors: saviors.clone() });
                        }
                    }
                    Act::Investigate => {
                        // Investigations do not occur if cop is dead
                        if kills.contains_key(&na.actor) {
                            continue;
                        }
                        let role = self.players.get(na.target);
                        if na.blockers.is_empty() {
                            self.tx(Event::Investigate {
                                cop: na.actor,
                                target: na.target,
                                appears_mafia: role.is_mafia(),
                            });
                        } else {
                            self.tx(Event::Block {
                                blocked: na.actor,
                                blockers: na.blockers.clone(),
                            });
                        }
                    }
                    Act::Milk => {
                        // Milk delivery does not happen if target is dead
                        if kills.contains_key(&na.target) {
                            continue;
                        }
                        if na.blockers.is_empty() {
                            self.tx(Event::Milk { milky: na.actor, target: na.target });
                        } else if self.players.is_alive(na.actor) {
                            self.tx(Event::Block {
                                blocked: na.actor,
                                blockers: na.blockers.clone(),
                            });
                        }
                    }
                }
            }
            if kills.is_empty() {
                self.tx(Event::NoKill);
            }
            for (mark, killer) in kills.into_iter() {
                self.tx(Event::Kill { killer, mark });
                self.eliminate(mark, killer, Context::new(self.day, Cause::Kill));
            }
            blocks
        }
    }
}
/*
#[cfg(test)]
mod tests {
    use core::panic;
    use std::f32::consts::E;

    use tracing_test::traced_test;

    use super::*;
    fn pid(n: u64) -> Pid {
        Pid::from(n)
    }

    fn state_3() -> State {
        State {
            day: 1,
            phase: Phase::Day { votes: HashMap::new(), blocks: HashMap::new(), elect: None },
            players: Players::from_registry(vec![
                (1, Role::TOWN),
                (2, Role::TOWN),
                (3, Role::MAFIA),
            ]),
            rules: Rules::default(),
            event_log: Vec::new(),
            event_tx: tokio::sync::broadcast::channel(100).0,
        }
    }

    #[test]
    fn vote_pass() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);

        let mut state = state_3();

        state.vote(1, Some(Some(1))).expect("Vote self should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(one));
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, None).expect("Unvote should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one), None);
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, Some(Some(2))).expect("Vote other should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(two));
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, Some(Some(3))).expect("Vote again should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(three));
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, Some(None)).expect("Vote none should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &None);
        } else {
            panic!("Expected Day phase");
        }
    }

    #[test]
    fn vote_fail() {
        // Votes in wrong phase
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let mut state = state_3();

        let err = state.vote(1, Some(Some(2))).expect_err("Vote in init should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };

        let err = state.vote(1, Some(Some(3))).expect_err("Vote in night should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::End { winner: Team::Town };

        let err = state.vote(1, Some(None)).expect_err("Vote in end should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day { votes: HashMap::new(), blocks: HashMap::new(), elect: None };

        // Invalid voter
        let err = state.vote(4, Some(Some(2))).expect_err("Invalid voter should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Invalid ballot

        let err = state.vote(1, Some(Some(4))).expect_err("Invalid ballot should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Ineffective vote

        state.vote(1, Some(Some(2))).expect("Vote should work");
        let err = state.vote(1, Some(Some(2))).expect_err("Ineffective vote should fail");

        assert!(matches!(err, Error::IneffectiveAction));
    }

    #[test]
    fn elect_update() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let mut state = state_3();

        state.vote(1, Some(Some(3))).expect("Vote self should work");
        state.vote(2, Some(Some(3))).expect("Vote self should work");

        // Now, after one update, election should be scheduled
        state.update();
        if let Phase::Day { elect: pend_elect, .. } = &mut state.phase {
            assert!(pend_elect.is_some());
            *pend_elect =
                Some((Some(Pid::from(3)), Pid::from(2), Local::now() - Duration::from_secs(1)));
        } else {
            panic!("Expected Day phase");
        }
        // After another update, with time changed, election should be done
        state.update();

        if let Phase::End { winner } = &state.phase {
            assert_eq!(winner, &Team::Town);
        } else {
            panic!("Expected End phase");
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn target_pass() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let four = Pid::from(4);
        let mut state = start_state_6();
        state.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };

        state.target(2, Some(4)).expect("Target self should work");
        if let Phase::Night { targets, .. } = &state.phase {
            assert_eq!(targets.get(&two).unwrap(), &Some(four));
        } else {
            panic!("Expected Night phase");
        }

        state.target(2, Some(3)).expect("Change target should work");
        if let Phase::Night { targets, .. } = &state.phase {
            assert_eq!(targets.get(&two).unwrap(), &Some(three));
        } else {
            panic!("Expected Night phase");
        }

        state.target(2, None).expect("Change to None Target should work");
        if let Phase::Night { targets, .. } = &state.phase {
            assert_eq!(targets.get(&two).unwrap(), &None);
        } else {
            panic!("Expected Night phase");
        }

        state.target(3, Some(2)).expect("Target other should work");

        state.scheme(4, Some(1)).expect("Scheme should work");

        state.target(5, Some(6)).expect("Stripper should work");
        state.target(6, Some(2)).expect("Milk should work");

        state.update(); // Dawn should be scheduled

        state.target(2, Some(4)).expect("Target should work even with dawn pending");

        if let Phase::Night { dawn, .. } = &mut state.phase {
            assert!(pend_dawn.is_some());
        } else {
            panic!("Expected Night phase");
        }
    }

    #[test]
    fn target_fail() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let four = Pid::from(4);
        let mut state = start_state_6();
        state.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };

        let err = state.target(7, Some(1)).expect_err("Invalid actor should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        let err = state.target(2, Some(7)).expect_err("Invalid target should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        let err = state.target(1, Some(4)).expect_err("Invalid role should fail");

        assert!(matches!(err, Error::ExpectedTargetingRole { actual: RoleKind::TOWN }));

        state.phase = Phase::Init;

        let err = state.target(2, Some(4)).expect_err("Target in init should fail");

        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day { votes: HashMap::new(), blocks: HashMap::new(), elect: None };

        let err = state.target(2, Some(4)).expect_err("Target in day should fail");

        assert!(matches!(err, Error::InvalidPhase { actual: PhaseKind::Day, .. }));
    }

    fn start_state_6() -> State {
        State::new(
            vec![
                (1, Role::TOWN),
                (2, Role::COP),
                (3, Role::DOCTOR),
                (4, Role::MAFIA),
                (5, Role::STRIPPER),
                (6, Role::MILKY),
            ],
            Rules::default(),
        )
    }

    #[test]
    fn test_dawn1() {
        let start_state = start_state_6();
        // successful save, block cop
        let mut state1 = start_state.clone();
        state1.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(4))),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(2))),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(3)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state1.update();

        assert!(matches!(state1.phase, Phase::Day { .. }));
        assert!(matches!(state1.players.n(), 6));

        let events = &state1.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Dawn,
            Event::Block { blocked: pid(2), blockers: vec![pid(5)] },
            Event::Save { saved: pid(3), saviors: vec![pid(3)] },
            Event::Milk { milky: pid(6), target: pid(1) },
            Event::NoKill,
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 4), (Team::Mafia, 2)].into_iter().collect(),
            },
        ];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn2() {
        let mut state = start_state_6();
        // no save, investigation and milked killed
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(4))),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(1))),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(1)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Investigate { cop: pid(2), target: pid(4), appears_mafia: true },
            Event::Kill { killer: pid(4), mark: pid(1) },
        ];
        let exp_not_events = vec![Event::Kill { killer: pid(4), mark: pid(3) }];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn3() {
        let mut state = start_state_6();
        // block save and kill doc
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), None),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(3))),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(3)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Kill { killer: pid(4), mark: pid(3) },
            Event::Block { blocked: pid(3), blockers: vec![pid(5)] },
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 3), (Team::Mafia, 2)].into_iter().collect(),
            },
            Event::Milk { milky: pid(6), target: pid(1) },
        ];
        let exp_not_events = vec![Event::NoKill];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn4() {
        let mut state = start_state_6();
        // cop killed, ineffective save blocked
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(1))),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(3))),
                (pid(6), None),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(2)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![Event::Kill { killer: pid(4), mark: pid(2) }];
        let exp_not_events = vec![
            Event::Investigate { cop: pid(2), target: pid(1), appears_mafia: false },
            Event::Block { blocked: pid(3), blockers: vec![pid(5)] },
        ];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn5() {
        let mut state = start_state_6();
        // posthumous milk, dead investigated target
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(6))),
                (pid(3), Some(pid(2))),
                (pid(5), None),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(6)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Kill { killer: pid(4), mark: pid(6) },
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 3), (Team::Mafia, 2)].into_iter().collect(),
            },
            Event::Milk { milky: pid(6), target: pid(1) },
            Event::Investigate { cop: pid(2), target: pid(6), appears_mafia: false },
        ];
        let exp_not_events = vec![];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    fn start_state_7() -> State {
        State::new(
            vec![
                (1, Role::MILLER),
                (2, Role::COP),
                (3, Role::DOCTOR),
                (4, Role::CELEB),
                (5, Role::IDIOT),
                (6, Role::GODFATHER),
                (7, Role::STRIPPER),
            ],
            Rules::default(),
        )
    }

    #[test]
    #[traced_test]
    fn test_eclipse_basic() {
        let mut state = start_state_7();

        state.phase = Phase::Day {
            votes: vec![(pid(1), Some(pid(5))), (pid(2), Some(pid(5))), (pid(3), Some(pid(5)))]
                .into_iter()
                .collect(),
            blocks: HashMap::new(),
            elect: None,
        };

        state.vote(5, Some(Some(5))).expect("Vote self should work");

        state.update();

        if let Phase::Day { elect, .. } = &mut state.phase {
            let Some((choice, hammer, time)) = &elect else {
                return panic!("Expected pending election");
            };
            let new_elect = Some((*choice, *hammer, *time - Duration::from_secs(100)));
            *elect = new_elect;
        } else {
            panic!("Expected Day phase");
        }

        state.update();

        if let Phase::Eclipse { avenger, hammer, guilty, vote } = &state.phase {
            assert_eq!(avenger, &pid(5));
            assert_eq!(hammer, &pid(5));
            for v in vec![pid(1), pid(2), pid(3), pid(5)] {
                assert!(guilty.contains(&v));
            }
            assert_eq!(vote, &None);
        } else {
            panic!("Expected Eclipse phase");
        }

        // Check fail votes during eclipse
        let err =
            state.vote(1, Some(Some(2))).expect_err("Voter not avenger during eclipse should fail");
        assert!(matches!(err, Error::IneffectiveAction));

        let err =
            state.vote(5, Some(Some(6))).expect_err("Avenger voting for not guilty should fail");
        assert!(matches!(err, Error::IneffectiveAction));

        let err = state.vote(5, None).expect_err("Avenger unvoting should fail");
        assert!(matches!(err, Error::IneffectiveAction));

        let err = state.vote(5, Some(None)).expect_err("Avenger voting for peace should fail");

        let err = state.vote(5, Some(Some(5))).expect_err("Avenger voting for self should fail");

        state.vote(5, Some(Some(1))).expect("Avenger voting for guilty should work");

        state.update();

        assert!(matches!(state.phase, Phase::Night { .. }));
        assert_eq!(state.players.n(), 5);

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Election {
                choice: Some(pid(1)),
                hammer: pid(5),
                voters: vec![pid(1), pid(2), pid(3), pid(5)],
            },
            Event::Eclipse {
                avenger: pid(5),
                hammer: pid(5),
                guilty: vec![pid(1), pid(2), pid(3), pid(5)],
            },
            Event::Vengeance { avenger: pid(5), victim: pid(1) },
            Event::Eliminate {
                player: pid(1),
                role: RoleKind::MILLER,
                context: Context::new(0, Cause::Vengeance),
            },
            Event::Eliminate {
                player: pid(5),
                role: RoleKind::IDIOT,
                context: Context::new(0, Cause::Election),
            },
        ];
    }
}
*/
