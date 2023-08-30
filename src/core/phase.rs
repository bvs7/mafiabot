use serde::Serialize;
use serenity::client::bridge::gateway::event;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Debug;
use std::usize;

use kinded::Kinded;

use super::interface::{ActionKind, Event, EventOutput, InvalidActionError};
use super::*;

type Pidx = usize;

type PlayerList = Vec<Player>;

#[derive(Debug, Clone, Kinded, Serialize)]
#[kinded(derive(Serialize))]
pub enum Phase {
    Init,
    Day(Day),
    Dusk(Dusk),
    Night(Night),
    End(End),
}

trait Tally: IntoIterator<Item = (Pidx, Option<Pidx>)> + Clone {
    fn tally(&self, choice: &Option<Pidx>) -> Vec<Pidx> {
        // Clones reference to self, so it can be turned into iterator
        self.clone()
            .into_iter()
            .filter_map(|(p, v)| (&v == choice).then(|| p))
            .collect()
    }
}

pub type Votes = HashMap<Pidx, Option<Pidx>>;
impl Tally for Votes {}

pub type Blocked = Vec<Pidx>;

#[derive(Debug, Clone, Serialize)]
pub struct Day {
    num: usize,
    players: PlayerList,
    votes: Votes,
    blocked: Blocked,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dusk {
    num: usize,
    players: PlayerList,
    avenger: Pidx,
    voters: Vec<Pidx>,
}

pub type Targets = HashMap<Pidx, Option<Pidx>>;
pub type Scheme = Option<(Pidx, Option<Pidx>)>;

#[derive(Debug, Clone, Serialize)]
pub struct Night {
    num: usize,
    players: PlayerList,
    targets: Targets,
    scheme: Scheme,
}

#[derive(Debug, Clone, Serialize)]
pub struct End {
    players: PlayerList,
    winner: Option<Team>,
}

// ***

pub trait FindPlayer: IntoIterator<Item = Player> + Clone {
    fn find(&self, pid: PID) -> Result<Pidx, InvalidActionError> {
        self.clone()
            .into_iter()
            .position(|p| p.user_id == pid)
            .ok_or_else(|| InvalidActionError::PlayerNotFound { pid })
    }
}

impl FindPlayer for PlayerList {}

#[derive(Debug, Clone, Serialize)]
enum ChoiceIdx {
    Choice(Option<Pidx>),
    Retract,
}

impl Choice {
    fn to_choice_idx<F: FnOnce(PID) -> Result<Pidx, InvalidActionError>>(
        &self,
        pid_to_pidx: F,
    ) -> Result<ChoiceIdx, InvalidActionError> {
        match self {
            Self::Player(p) => pid_to_pidx(*p).map(|p| ChoiceIdx::Choice(Some(p))),
            Self::Abstain => Ok(ChoiceIdx::Choice(None)),
            Self::None => Ok(ChoiceIdx::Retract),
        }
    }
}

impl ChoiceIdx {
    /// Turn a choiceIdx into a choice, given a function to convert a player index into a PID.
    fn to_choice<F: FnOnce(Pidx) -> PID>(&self, pidx_to_pid: F) -> Choice {
        match self {
            Self::Choice(c) => match c {
                Some(p) => Choice::Player(pidx_to_pid(*p)),
                None => Choice::Abstain,
            },
            Self::Retract => Choice::None,
        }
    }
}

fn check_win(players: &PlayerList) -> Option<Team> {
    // Count up mafia
    let n_players = players.len();
    let n_mafia = players
        .iter()
        .filter(|p| p.role.team() == Team::Mafia)
        .count();

    if n_mafia == 0 {
        return Some(Team::Town);
    } else if n_mafia >= n_players - n_mafia {
        return Some(Team::Mafia);
    }
    return None;
}

impl Day {
    fn from_night(night: Night, blocked: Blocked) -> Self {
        Self {
            num: night.num + 1,
            players: night.players,
            votes: Votes::new(),
            blocked: blocked,
        }
    }

    pub fn vote(
        &mut self,
        voter: PID,
        choice: Choice,
        event_output: &EventOutput,
    ) -> Result<bool, InvalidActionError> {
        // Validate vote
        let voter_idx = self.players.find(voter)?;
        let choice_idx = choice.to_choice_idx(|p| self.players.find(p))?;
        let former_idx = match choice_idx {
            ChoiceIdx::Retract => self.votes.remove(&voter_idx),
            ChoiceIdx::Choice(c) => self.votes.insert(voter_idx, c),
        };

        let former = match former_idx {
            None => Choice::None,
            Some(None) => Choice::Abstain,
            Some(Some(votee_idx)) => Choice::Player(self.players[votee_idx].user_id),
        };

        match choice_idx {
            ChoiceIdx::Retract => {
                // Send retract event
                event_output
                    .send(Event::Retract {
                        voter,
                        former: former,
                    })
                    .expect("Failed to send event");
            }
            ChoiceIdx::Choice(c) => {
                let n_players = self.players.len();
                let threshold = match c {
                    Some(_) => n_players / 2 + 1,
                    None => (n_players + 1) / 2,
                };
                let voters: Vec<_> = self
                    .votes
                    .iter()
                    .filter_map(|(p, v)| (*v == c).then(|| *p))
                    .collect();
                let voters = self.votes.tally(&c);
                let count = voters.len();
                event_output
                    .send(Event::Vote {
                        voter,
                        choice,
                        former,
                        threshold,
                        count,
                    })
                    .expect("Failed to send event");

                return Ok(count >= threshold);
            }
        }
        Ok(false)
    }

    pub fn reveal(
        &self,
        actor: PID,
        event_output: &EventOutput,
    ) -> Result<bool, InvalidActionError> {
        let actor_idx = self.players.find(actor)?;
        let actor_role = self.players[actor_idx].role;
        if actor_role != Role::CELEB {
            return Err(InvalidActionError::InvalidRole {
                role: actor_role,
                action: ActionKind::Reveal,
            });
        }
        if self.blocked.contains(&actor_idx) {
            event_output
                .send(Event::Block { blocked: actor })
                .expect("Failed to send event");
            return Ok(false);
        }
        event_output
            .send(Event::Reveal { celeb: actor })
            .expect("Failed to send event");
        return Ok(true);
    }

    fn threshold(&self, c: Option<Pidx>) -> usize {
        match c {
            Some(_) => self.players.len() / 2 + 1,
            None => (self.players.len() + 1) / 2,
        }
    }

    /// Check votes for a valid election state. Returns the elected player and voters
    fn check_election(&self) -> Option<(Option<Pidx>, Vec<Pidx>)> {
        // Create mapping of candidates to their voters
        let mut vote_map: HashMap<Option<Pidx>, Vec<Pidx>> = HashMap::new();
        for (voter, ballot) in self.votes.iter() {
            let entry = vote_map.entry(*ballot).or_insert(Vec::new());
            entry.push(*voter);
        }

        // Find the candidate with the most votes
        let mut heap = BinaryHeap::with_capacity(vote_map.len());
        for (candidate, voters) in vote_map.iter() {
            heap.push((voters.len(), candidate.is_none(), candidate, voters));
        }

        let (count, _, candidate, voters) = heap.pop()?;

        let threshold = self.threshold(*candidate);
        if count > threshold {
            return Some((*candidate, voters.to_owned()));
        }

        return None;
    }

    /// If votes are in a valid election state, progress the game
    pub fn try_elect(mut self, event_output: &EventOutput) -> Phase {
        let (elected_idx, electors_idx) = match self.check_election() {
            Some((elected, voters)) => (elected, voters),
            None => return Phase::Day(self),
        };

        let elected = elected_idx.map(|p| self.players[p].user_id);
        let electors = electors_idx
            .iter()
            .map(|p| self.players[*p].user_id)
            .collect();

        event_output
            .send(Event::Election { electors, elected })
            .expect("Failed to send event");

        // Check elected role
        if let Some(p_idx) = elected_idx {
            if self.players[p_idx].role == Role::IDIOT {
                return Phase::Dusk(Dusk::from_day(self, p_idx, electors_idx));
            }

            // Eliminate player
            eliminate(&mut self.players, p_idx, event_output);

            // Check if game is over
            if let Some(winner) = check_win(&self.players) {
                return Phase::End(End {
                    players: self.players,
                    winner: Some(winner),
                });
            }

            return Phase::Night(self.into());
        }
        return Phase::Day(self);
    }
}

pub fn eliminate(players: &mut PlayerList, p_idx: Pidx, event_output: &EventOutput) {
    let player = players.remove(p_idx);
    let eliminated = player.user_id;
    let role = player.role;

    event_output
        .send(Event::Eliminate { eliminated, role })
        .expect("Failed to send event");
}

impl Dusk {
    fn from_day(day: Day, avenger: Pidx, voters: Vec<Pidx>) -> Self {
        Self {
            num: day.num,
            players: day.players,
            avenger,
            voters,
        }
    }

    pub fn revenge(
        &mut self,
        choice: PID,
        event_output: &EventOutput,
    ) -> Result<Phase, InvalidActionError> {
        return Err(InvalidActionError::NoGame);
    }
}

impl From<Day> for Night {
    fn from(day: Day) -> Self {
        Self {
            num: day.num,
            players: day.players,
            targets: Targets::new(),
            scheme: None,
        }
    }
}

impl From<Dusk> for Night {
    fn from(dusk: Dusk) -> Self {
        Self {
            num: dusk.num,
            players: dusk.players,
            targets: Targets::new(),
            scheme: None,
        }
    }
}

impl Night {
    pub fn target(
        &mut self,
        actor: PID,
        target: Choice,
        event_output: &EventOutput,
    ) -> Result<bool, InvalidActionError> {
        let actor_idx = self.players.find(actor)?;
        let target_idx = target.to_choice_idx(|p| self.players.find(p))?;
        let actor_role = self.players[actor_idx].role;
        if !actor_role.targeting() {
            return Err(InvalidActionError::InvalidRole {
                role: actor_role,
                action: ActionKind::Target,
            });
        }
        // if actor is trying to target twice...
        // a.k.a. is, target not Abstain and scheme killer is actor and not Abstain
        match (&target_idx, &self.scheme) {
            (ChoiceIdx::Choice(Some(_)), Some((killer_idx, Some(_))))
                if *killer_idx == actor_idx =>
            {
                self.scheme = None;
                event_output
                    .send(Event::Mark {
                        killer: actor,
                        mark: Choice::None,
                    })
                    .expect("Failed to send event");
            }
            _ => (),
        }
        match &target_idx {
            ChoiceIdx::Choice(c) => self.targets.insert(actor_idx, *c),
            ChoiceIdx::Retract => self.targets.remove(&actor_idx),
        };
        event_output
            .send(Event::Target {
                actor,
                target: target,
            })
            .expect("Failed to send event");

        return Ok(self.check_done());
    }

    pub fn mark(
        &mut self,
        killer: PID,
        mark: Choice,
        event_output: &EventOutput,
    ) -> Result<bool, InvalidActionError> {
        let killer_idx = self.players.find(killer)?;
        let mark_idx = mark.to_choice_idx(|p| self.players.find(p))?;
        let killer_role = self.players[killer_idx].role;
        if !killer_role.marking() {
            return Err(InvalidActionError::InvalidRole {
                role: killer_role,
                action: ActionKind::Mark,
            });
        }

        // Check if killer has already targeted
        let killer_target = self.targets.get(&killer_idx);
        match (&mark_idx, killer_target) {
            (ChoiceIdx::Choice(Some(_)), Some(Some(_))) => {
                // Reset killer's target
                self.targets.remove(&killer_idx);
                event_output
                    .send(Event::Target {
                        actor: killer,
                        target: Choice::None,
                    })
                    .expect("Failed to send event");
            }
            _ => (),
        };

        match &mark_idx {
            ChoiceIdx::Choice(c) => self.scheme = Some((killer_idx, *c)),
            ChoiceIdx::Retract => self.scheme = None,
        };
        event_output
            .send(Event::Mark { killer, mark })
            .expect("Failed to send event");

        return Ok(self.check_done());
    }

    fn check_done(&self) -> bool {
        let targeting_done = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.role.targeting())
            .all(|(p_idx, _)| self.targets.contains_key(&p_idx));
        let scheme_done = self.scheme.is_some();
        return targeting_done && scheme_done;
    }

    fn try_dawn(self) -> Phase {
        // If night is done, progress to dawn

        if !self.check_done() {
            return Phase::Night(self);
        }

        // Collect Stripper targets

        // Block Stripper targets

        // Collect Doctor targets

        // Save Doctor targets

        // Collect Cop targets

        // Investigate Cop targets

        // Try Mafia Kill

        Phase::Day(Day::from_night(self, Vec::new()))
    }
}
