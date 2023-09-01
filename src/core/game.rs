use std::collections::{HashMap, HashSet};

use super::*;

use kinded::Kinded;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Kinded)]
#[kinded(derive(Serialize))]
pub enum Phase {
    Day {
        /// Mapping votee to voter?
        votes: HashMap<PID, Choice>,
        blocks: HashSet<PID>,
    },
    Night {
        targets: HashMap<PID, Choice>,
        scheme: Option<(PID, Choice)>,
    },
    Dusk {
        avenger: PID,
        voters: HashSet<PID>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Kinded)]
pub enum State {
    Init,
    Play {
        day_num: usize,
        players: PIDs,
        // Current roles in game
        roles: RoleAssign,
        phase: Phase,
    },
    End {
        winner: Option<Team>,
        survivors: PIDs,
        contracts: Vec<Contract>,
    },
}

pub struct Game {
    /// Unique identifier for this game
    game_id: usize,
    /// Game State Data
    state: State,
    rules: GameRules,
    /// A History of players and the roles assigned to them
    role_history: RoleHistory,
    /// Output queue generated Events are pushed to
    event_output: EventOutput,
}

use CoreError::*;

fn vote(
    voter: PID,
    ballot: Option<Choice>,
    players: &PIDs,
    votes: &mut HashMap<PID, Choice>,
    event_output: &EventOutput,
) -> Result<(), CoreError> {
    match ballot {
        Some(choice) => {
            if let Choice::Player(votee) = &choice {
                players.check(votee)?;
            }
            votes.insert(voter, choice);
        }
        None => {
            // Unvote
            votes.remove(&voter);
        }
    }
    event_output.send(Event::Vote { voter, ballot });

    Ok(())
}

impl Game {
    fn handle_vote(&mut self, voter: PID, ballot: Option<Choice>) -> Result<(), CoreError> {
        // Validate State
        if let State::Play { phase, players, .. } = &mut self.state {
            // Validate Phase
            if let Phase::Day { votes, .. } = phase {
                vote(voter, ballot, players, votes, &self.event_output)
            } else {
                Err(InvalidPhase(PhaseKind::Day, phase.kind()))
            }
        } else {
            Err(InvalidState(StateKind::Play, self.state.kind()))
        }
    }
}
