use chrono::{DateTime, Local};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

mod night_act;
mod phase;
mod players;

use super::*;
use night_act::NightAct;
pub use phase::PhaseKind;
use phase::{Blocks, Phase, Votes};
use players::{PlayerState, Players};


pub struct Status {
    pub day: u32,
    pub phase: PhaseKind,
    pub votes: Option<HashMap<Choice, Vec<Pid>>>,
    pub count: HashMap<Team, usize>,
    pub players: Vec<Pid>,
    pub rules: Rules,
    pub names: HashMap<Pid, String>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    #[serde(skip)]
    event_tx: mpsc::UnboundedSender<Event>,
    #[serde(skip)]
    action_rx: mpsc::Receiver<(Action, oneshot::Sender<Result<(), Error>>)>,
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
}

pub enum ValidAction {
    Vote(Pid, Ballot),
    Reveal(Pid),
    Target(Pid, Choice),
    Scheme(Pid, Choice),
    EclipseVote(Pid),
}

impl State {
    pub fn validate_action(&self, action: &Action) -> Result<ValidAction, Error> {
        use Action::*;
        let actor = match action {
            Vote { voter: actor, .. } |
            Target { actor, .. } |
            Scheme { killer: actor, .. } |
            Reveal { actor: actor } => self.players.validate(actor),
        }
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
}

enum ElectionResult {
    Some(Pid, Pid),
    Peace,
    Idiot(Pid, Pid, Vec<Pid>),
}

impl State {
    pub fn update(&mut self) -> Option<DateTime<Local>> {
        match &self.phase {
            Phase::Day { elect: Some((choice, hammer, time)), votes, .. } => {
                if time < &Local::now() {
                    match self.election(*choice, *hammer) {
                        ElectionResult::Some(pid, hammer) => {
                            self.eliminate(pid, hammer, Context::new(self.day, Cause::Election));
                        }
                        ElectionResult::Peace => {}
                        ElectionResult::Idiot(pid, hammer, voters) => {
                            self.eclipse(pid, hammer, voters);
                            return None;
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

    fn election(&mut self, choice: Choice, hammer: Pid) -> ElectionResult {
        let Phase::Day { votes, elect: Some((choice, hammer, _)), .. } = &self.phase else {
            panic!("Expected Day phase with some election");
        };
        let vote_list = count_votes(votes, &self.players);
        let th = thresh(self.players.alive().len(), &choice);
        let ph = thresh(self.players.alive().len(), &None);
        let voters = count_votes(votes, &self.players).get(choice).unwrap().clone();
        let (choice, hammer) = (*choice, *hammer);
        self.tx(Event::Election { choice, hammer, voters: voters.clone() });
        if let Some(pid) = choice {
            if self.players.get(pid) == Role::IDIOT {
                return ElectionResult::Idiot(pid, hammer, voters);
            } else {
                return ElectionResult::Some(pid, hammer);
            }
        }
        return ElectionResult::Peace;
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
}
