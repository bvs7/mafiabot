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
    id::{Choice, Gid, Pid, RawBallot, RawChoice},
    phase::{Blocks, Phase, PhaseKind, Votes},
    players::{Cause, Context, PlayerState, Players},
    Error, Event, Role, RoleKind, Rules, Team,
};

const ELECTION_DELAY: Duration = Duration::from_secs(10);
const DAWN_DELAY: Duration = Duration::from_secs(10);
const TICK_DELAY: Duration = Duration::from_secs(1);
/*
Have scheduled events? Or at least a thread that will notify when needed.
*/

fn count_votes(votes: &Votes) -> HashMap<Choice, Vec<Pid>> {
    let mut vote_list: HashMap<Choice, Vec<Pid>> = HashMap::new();
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
            Event::Vote {
                voter,
                ballot: Some((c, _)),
                ..
            } => {
                if c == choice {
                    return Some(*voter);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Clone)]
pub struct State_ {
    inner: Arc<Mutex<(State, Option<JoinHandle<()>>)>>,
}

impl State_ {
    pub fn new(registry: impl IntoIterator<Item = (u64, Role)>, rules: Rules) -> Self {
        let inner = Arc::new(Mutex::new((State::new(registry, rules), None)));
        Self { inner }
    }

    pub fn wake(&self) {
        if let Some(handle) = &self.inner.lock().unwrap().1 {
            handle.thread().unpark();
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn run(&self) {
        loop {
            park_timeout(TICK_DELAY);
            let mut lock = match self.inner.lock() {
                Ok(s) => s,
                Err(err) => {
                    error!("Failed to lock state: {:?}", err);
                    return;
                }
            };

            lock.0.update();
        }
    }

    /// Used in lieu of a started update thread
    pub fn update(&self) {
        let mut lock = self.inner.lock().unwrap();
        lock.0.update();
    }

    pub fn start_update_thread(&self) {
        let mut inner_lock = self.inner.lock().unwrap();
        if inner_lock.1.is_some() {
            return; // Already started
        }
        let s2 = self.clone();
        let handle = std::thread::spawn(move || {
            s2.run();
        });
        inner_lock.1 = Some(handle);
    }
    pub fn start(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.0.start();
    }
    pub fn vote(&self, voter: u64, ballot: RawBallot) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        let result = inner.0.vote(voter, ballot);
        self.wake();
        result
    }

    pub fn reveal(&self, celeb: u64) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        inner.0.reveal(celeb)
    }

    pub fn target(&self, actor: u64, choice: RawChoice) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        let result = inner.0.target(actor, choice);
        self.wake();
        result
    }

    pub fn scheme(&self, killer: u64, mark: RawChoice) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        let result = inner.0.scheme(killer, mark);
        self.wake();
        result
    }
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

// TODO: have event log be an generic trait, so that an implementation can define it?
#[derive(Debug, Clone)]
pub struct State {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    event_log: Vec<Event>,
    event_tx: tokio::sync::broadcast::Sender<Event>,
}

impl State {
    pub fn new(registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>, rules: Rules) -> Self {
        Self {
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_log: Vec::new(),
            event_tx: tokio::sync::broadcast::channel(100).0,
            // update_thread: None,
        }
    }

    pub fn get_target(&self, idx: usize) -> Result<Choice, Error> {
        self.players.get_target(idx)
    }

    pub fn log_event(&mut self, event: Event) {
        self.event_log.push(event.clone());
        let _ = self.event_tx.send(event);
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

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.event_tx.subscribe()
    }

    fn start(&mut self) {
        self.log_event(Event::Start {
            players: self.players.alive(),
            rules: self.rules.clone(),
        });
        let n = self.players.n();
        if n % 2 == 1 {
            self.day(HashMap::new());
        } else {
            self.night();
        }
    }

    pub fn vote(&mut self, voter: u64, ballot: RawBallot) -> Result<(), Error> {
        let voter = self.players.validate(voter)?;
        let ballot = self.players.validate_ballot(ballot)?;
        if matches!(self.phase, Phase::Eclipse { .. }) {
            return self.phase.eclipse_vote(voter, ballot);
        }
        let former = self.phase.vote(voter, ballot)?;
        let mut vote_list = self.phase.vote_list()?;

        let ballot = ballot.map(|c| (c, vote_list.entry(c).or_default().len()));
        let former = former.map(|c| (c, vote_list.entry(c).or_default().len()));

        self.log_event(Event::Vote {
            voter,
            ballot,
            former,
        });
        Ok(())
    }

    pub fn reveal(&mut self, celeb: u64) -> Result<(), Error> {
        let celeb = self.players.validate(celeb)?;
        let role = self.players.get(celeb);
        if role != Role::CELEB {
            return Err(Error::ExpectedCeleb {
                actual: role.kind(),
            });
        }
        let Phase::Day { blocks, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Day);
        };
        if let Some(blockers) = blocks.get(&celeb) {
            self.log_event(Event::Block {
                blocked: celeb,
                blockers: blockers.clone(),
            });
        } else {
            // Check if celeb is blocked
            self.log_event(Event::Reveal { celeb });
        }
        Ok(())
    }

    pub fn target(&mut self, actor: u64, choice: RawChoice) -> Result<(), Error> {
        let actor = self.players.validate(actor)?;
        let choice = self.players.validate_choice(choice)?;
        let role = self.players.get(actor);
        if !role.is_targeting() {
            return Err(Error::ExpectedTargetingRole {
                actual: role.kind(),
            });
        }
        self.phase.target(actor, choice)?;
        self.log_event(Event::Target { actor, choice });
        Ok(())
    }

    pub fn scheme(&mut self, killer: u64, mark: RawChoice) -> Result<(), Error> {
        let killer = self.players.validate(killer)?;
        let mark = self.players.validate_choice(mark)?;
        let role = self.players.get(killer);
        if !role.is_scheming() && mark.is_some() {
            return Err(Error::ExpectedSchemingRole {
                actual: role.kind(),
            });
        }
        self.phase.scheme(killer, mark)?;
        self.log_event(Event::Scheme { killer, mark });
        Ok(())
    }

    fn elect(&mut self, choice: Choice, hammer: Pid, voters: Vec<Pid>) -> bool {
        self.log_event(Event::Election {
            choice,
            hammer,
            voters: voters.clone(),
        });
        if let Some(pid) = choice {
            // Check for IDIOT
            if self.players.get(pid) == Role::IDIOT {
                self.eclipse(pid, hammer, voters);
                return true;
            } else {
                self.eliminate(pid, hammer, Context::new(self.day, Cause::Election));
            }
        }
        return false;
    }

    fn eclipse(&mut self, avenger: Pid, hammer: Pid, guilty: Vec<Pid>) {
        self.phase = Phase::Eclipse {
            avenger,
            hammer,
            guilty: guilty.clone(),
            vote: None,
        };
        self.log_event(Event::Eclipse {
            avenger,
            hammer,
            guilty,
        });
    }
    fn vengeance(&mut self, avenger: Pid, victim: Pid, hammer: Pid) {
        self.log_event(Event::Vengeance { avenger, victim });
        self.eliminate(victim, avenger, Context::new(self.day, Cause::Vengeance));
        self.eliminate(avenger, hammer, Context::new(self.day, Cause::Election));
    }

    fn dawn(&mut self) -> Blocks {
        self.log_event(Event::Dawn);
        let night_actions = NightAct::from_state(self);

        self.apply_night_actions(night_actions)
    }

    fn eliminate(&mut self, player: Pid, _culpable: Pid, context: Context) {
        let PlayerState::Alive(role) = self.players.eliminate(&player, context) else {
            panic!("Eliminating a dead player?");
        };
        let role = role.kind();
        self.log_event(Event::Eliminate {
            player,
            role,
            context,
        });
    }

    fn day(&mut self, blocks: Blocks) {
        self.day += 1;
        self.phase = Phase::Day {
            votes: HashMap::new(),
            blocks,
            pend_elect: None,
        };
        let counts = self.players.counts(Team::from);
        self.log_event(Event::Day {
            day: self.day,
            counts,
        });
    }

    fn night(&mut self) {
        self.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };
        let counts = self.players.counts(Team::from);
        self.log_event(Event::Night {
            day: self.day,
            counts,
        });
    }

    fn check_end(&mut self) -> bool {
        let counts = self.players.counts(Team::from);
        let n = self.players.alive().len();
        let n_maf = counts.get(&Team::Mafia).copied().unwrap_or(0);
        if n_maf == 0 {
            self.phase = Phase::End { winner: Team::Town };
            return true;
        } else if n - n_maf <= n_maf {
            self.phase = Phase::End {
                winner: Team::Mafia,
            };
            return true;
        }
        false
    }

    /*
    One big idea: if we just assume that this update function is called at the right
    times, we know things should work. We could just have the lobby call this function
    in its update loop. It might also be nice to have a function that tells us when
    the next timer wakeup will be. This way, we could have whatever is calling the
    update function know when it's best to call it next.

    One thing to think about, though, is polling events that emerge.
    If we did have the lobby update function handling events, and it has, say, 5 events
    to send out, each of which might take 1-2 seconds to send... that could be bad...

    This means we should probably separate the update function from the event handling.
    So the update function is in charge of... pretty much just checking if timers have
    ended, and calling update on the games that need it.
    We will also want one event handler for each game, that will poll the game's event
    log and send out events as needed.

    "Send event" in the state could be both adding it to the log and sending to the
    broadcast channel... or the log could be a broadcast listener. But that seems bad.
    If anything, the log should happen within the mutex.
     */

    /// Test for an update in phase, or for phase ending updates
    fn update(&mut self) -> Option<DateTime<Local>> {
        #[derive(Debug)]
        enum UpdateResult {
            /// Choice, Hammer, Voters
            Election(Choice, Pid, Vec<Pid>),
            Dawn,
            /// Avenger, Victim, Hammer
            Vengeance(Pid, Pid, Pid),
            ElectionImminent(DateTime<Local>),
            DawnImminent(DateTime<Local>),
        }
        let mut result = None;
        match &mut self.phase {
            Phase::Day {
                votes, pend_elect, ..
            } => {
                let mut vote_list: HashMap<Choice, Vec<Pid>> = HashMap::new();
                for (voter, choice) in votes {
                    vote_list.entry(*choice).or_default().push(*voter);
                }
                let n = self.players.n();
                if let Some((choice, hammer, time)) = pend_elect {
                    let voters = vote_list.entry(*choice).or_default();
                    let thresh = thresh(n, choice);
                    if voters.len() < thresh {
                        *pend_elect = None;
                    } else if *time < Local::now() {
                        result = Some(UpdateResult::Election(*choice, *hammer, voters.clone()));
                    }
                }
                if pend_elect.is_none() {
                    for (choice, voters) in vote_list {
                        let thresh = thresh(n, &choice);
                        if voters.len() >= thresh {
                            let hammer = last_vote_for(&self.event_log, &choice).unwrap();
                            let time = Local::now() + ELECTION_DELAY;
                            *pend_elect = Some((choice, hammer, time));
                            result = Some(UpdateResult::ElectionImminent(time));
                        }
                    }
                }
            }
            Phase::Night {
                pend_dawn: Some(time),
                ..
            } if *time < Local::now() => {
                result = Some(UpdateResult::Dawn);
            }
            Phase::Night {
                targets,
                scheme,
                pend_dawn,
            } => {
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
                    *pend_dawn = Some(time);
                    result = Some(UpdateResult::DawnImminent(time));
                }
            }
            Phase::Eclipse {
                avenger,
                hammer,
                vote: Some(victim),
                ..
            } => {
                result = Some(UpdateResult::Vengeance(*avenger, *victim, *hammer));
            }
            _ => {}
        }
        if let Some(result) = &result {
            tracing::info!("Got update result: {:?}", result);
        }
        match result {
            Some(UpdateResult::Election(choice, hammer, voters)) => {
                if !self.elect(choice, hammer, voters) {
                    if !self.check_end() {
                        self.night();
                    }
                }
            }
            Some(UpdateResult::Dawn) => {
                let blocks = self.dawn();
                if !self.check_end() {
                    self.day(blocks);
                }
            }
            Some(UpdateResult::Vengeance(avenger, victim, hammer)) => {
                self.vengeance(avenger, victim, hammer);
                if !self.check_end() {
                    self.night();
                }
            }
            Some(UpdateResult::ElectionImminent(time)) | Some(UpdateResult::DawnImminent(time)) => {
                return Some(time);
            }
            None => {}
        }
        None
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
            tracing::debug!(
                "Night action from target: {:?}, {:?}, {:?}",
                role,
                actor,
                target
            );
            use Role::*;
            match role {
                STRIPPER => NightAct {
                    act: Act::Block,
                    actor,
                    target,
                    blockers: vec![],
                },
                DOCTOR => NightAct {
                    act: Act::Save { effective: false },
                    actor,
                    target,
                    blockers: vec![],
                },
                COP => NightAct {
                    act: Act::Investigate,
                    actor,
                    target,
                    blockers: vec![],
                },
                MILKY => NightAct {
                    act: Act::Milk,
                    actor,
                    target,
                    blockers: vec![],
                },
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
            let Phase::Night {
                targets, scheme, ..
            } = &state.phase
            else {
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
                            self.log_event(Event::Block {
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
                            self.log_event(Event::Save {
                                saved: na.target,
                                saviors: saviors.clone(),
                            });
                        }
                    }
                    Act::Investigate => {
                        // Investigations do not occur if cop is dead
                        if kills.contains_key(&na.actor) {
                            continue;
                        }
                        let role = self.players.get(na.target);
                        if na.blockers.is_empty() {
                            self.log_event(Event::Investigate {
                                cop: na.actor,
                                target: na.target,
                                appears_mafia: role.is_mafia(),
                            });
                        } else {
                            self.log_event(Event::Block {
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
                            self.log_event(Event::Milk {
                                milky: na.actor,
                                target: na.target,
                            });
                        } else if self.players.is_alive(na.actor) {
                            self.log_event(Event::Block {
                                blocked: na.actor,
                                blockers: na.blockers.clone(),
                            });
                        }
                    }
                }
            }
            if kills.is_empty() {
                self.log_event(Event::NoKill);
            }
            for (mark, killer) in kills.into_iter() {
                self.log_event(Event::Kill { killer, mark });
                self.eliminate(mark, killer, Context::new(self.day, Cause::Kill));
            }
            blocks
        }
    }
}

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
            phase: Phase::Day {
                votes: HashMap::new(),
                blocks: HashMap::new(),
                pend_elect: None,
            },
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

        state
            .vote(1, Some(Some(2)))
            .expect("Vote other should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(two));
        } else {
            panic!("Expected Day phase");
        }

        state
            .vote(1, Some(Some(3)))
            .expect("Vote again should work");
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

        let err = state
            .vote(1, Some(Some(2)))
            .expect_err("Vote in init should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };

        let err = state
            .vote(1, Some(Some(3)))
            .expect_err("Vote in night should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::End { winner: Team::Town };

        let err = state
            .vote(1, Some(None))
            .expect_err("Vote in end should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day {
            votes: HashMap::new(),
            blocks: HashMap::new(),
            pend_elect: None,
        };

        // Invalid voter
        let err = state
            .vote(4, Some(Some(2)))
            .expect_err("Invalid voter should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Invalid ballot

        let err = state
            .vote(1, Some(Some(4)))
            .expect_err("Invalid ballot should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Ineffective vote

        state.vote(1, Some(Some(2))).expect("Vote should work");
        let err = state
            .vote(1, Some(Some(2)))
            .expect_err("Ineffective vote should fail");

        assert!(matches!(err, Error::IneffectiveVote));
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
        if let Phase::Day { pend_elect, .. } = &mut state.phase {
            assert!(pend_elect.is_some());
            *pend_elect = Some((
                Some(Pid::from(3)),
                Pid::from(2),
                Local::now() - Duration::from_secs(1),
            ));
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
        state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };

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

        state
            .target(2, None)
            .expect("Change to None Target should work");
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

        state
            .target(2, Some(4))
            .expect("Target should work even with dawn pending");

        if let Phase::Night { pend_dawn, .. } = &mut state.phase {
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
        state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };

        let err = state
            .target(7, Some(1))
            .expect_err("Invalid actor should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        let err = state
            .target(2, Some(7))
            .expect_err("Invalid target should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        let err = state
            .target(1, Some(4))
            .expect_err("Invalid role should fail");

        assert!(matches!(
            err,
            Error::ExpectedTargetingRole {
                actual: RoleKind::TOWN
            }
        ));

        state.phase = Phase::Init;

        let err = state
            .target(2, Some(4))
            .expect_err("Target in init should fail");

        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day {
            votes: HashMap::new(),
            blocks: HashMap::new(),
            pend_elect: None,
        };

        let err = state
            .target(2, Some(4))
            .expect_err("Target in day should fail");

        assert!(matches!(
            err,
            Error::InvalidPhase {
                actual: PhaseKind::Day,
                ..
            }
        ));
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
            pend_dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state1.update();

        assert!(matches!(state1.phase, Phase::Day { .. }));
        assert!(matches!(state1.players.n(), 6));

        let events = &state1.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Dawn,
            Event::Block {
                blocked: pid(2),
                blockers: vec![pid(5)],
            },
            Event::Save {
                saved: pid(3),
                saviors: vec![pid(3)],
            },
            Event::Milk {
                milky: pid(6),
                target: pid(1),
            },
            Event::NoKill,
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 4), (Team::Mafia, 2)]
                    .into_iter()
                    .collect(),
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
            pend_dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Investigate {
                cop: pid(2),
                target: pid(4),
                appears_mafia: true,
            },
            Event::Kill {
                killer: pid(4),
                mark: pid(1),
            },
        ];
        let exp_not_events = vec![Event::Kill {
            killer: pid(4),
            mark: pid(3),
        }];

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
            pend_dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Kill {
                killer: pid(4),
                mark: pid(3),
            },
            Event::Block {
                blocked: pid(3),
                blockers: vec![pid(5)],
            },
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 3), (Team::Mafia, 2)]
                    .into_iter()
                    .collect(),
            },
            Event::Milk {
                milky: pid(6),
                target: pid(1),
            },
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
            pend_dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![Event::Kill {
            killer: pid(4),
            mark: pid(2),
        }];
        let exp_not_events = vec![
            Event::Investigate {
                cop: pid(2),
                target: pid(1),
                appears_mafia: false,
            },
            Event::Block {
                blocked: pid(3),
                blockers: vec![pid(5)],
            },
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
            pend_dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Kill {
                killer: pid(4),
                mark: pid(6),
            },
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 3), (Team::Mafia, 2)]
                    .into_iter()
                    .collect(),
            },
            Event::Milk {
                milky: pid(6),
                target: pid(1),
            },
            Event::Investigate {
                cop: pid(2),
                target: pid(6),
                appears_mafia: false,
            },
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
            votes: vec![
                (pid(1), Some(pid(5))),
                (pid(2), Some(pid(5))),
                (pid(3), Some(pid(5))),
            ]
            .into_iter()
            .collect(),
            blocks: HashMap::new(),
            pend_elect: None,
        };

        state.vote(5, Some(Some(5))).expect("Vote self should work");

        state.update();

        if let Phase::Day { pend_elect, .. } = &mut state.phase {
            let Some((choice, hammer, time)) = &pend_elect else {
                return panic!("Expected pending election");
            };
            let new_elect = Some((*choice, *hammer, *time - Duration::from_secs(100)));
            *pend_elect = new_elect;
        } else {
            panic!("Expected Day phase");
        }

        state.update();

        if let Phase::Eclipse {
            avenger,
            hammer,
            guilty,
            vote,
        } = &state.phase
        {
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
        let err = state
            .vote(1, Some(Some(2)))
            .expect_err("Voter not avenger during eclipse should fail");
        assert!(matches!(err, Error::IneffectiveVote));

        let err = state
            .vote(5, Some(Some(6)))
            .expect_err("Avenger voting for not guilty should fail");
        assert!(matches!(err, Error::IneffectiveVote));

        let err = state
            .vote(5, None)
            .expect_err("Avenger unvoting should fail");
        assert!(matches!(err, Error::IneffectiveVote));

        let err = state
            .vote(5, Some(None))
            .expect_err("Avenger voting for peace should fail");

        let err = state
            .vote(5, Some(Some(5)))
            .expect_err("Avenger voting for self should fail");

        state
            .vote(5, Some(Some(1)))
            .expect("Avenger voting for guilty should work");

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
            Event::Vengeance {
                avenger: pid(5),
                victim: pid(1),
            },
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
