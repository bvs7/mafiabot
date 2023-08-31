use std::collections::{HashMap, HashSet};

use super::interface::InvalidActionError;
use super::{Contract, EventOutput, GameRules, PIDs, Player, Players, Role, RoleGen, Team, PID};

use kinded::Kinded;
use serde::Serialize;

type Pidx = usize;
type PlayerList = Vec<Player>;

// Entrants + RoleGen -> Players + Contracts

// pub struct Game {
//     game_id: usize,
//     phase: Phase,
//     contracts: Contracts,
//     entrants: PIDs,
//     rolegen: RoleGen,
//     rules: GameRules,
// }

pub trait CheckPlayer: IntoIterator<Item = PID> + Clone {
    fn check(&self, pid: PID) -> Result<PID, InvalidActionError> {
        self.clone()
            .into_iter()
            .find(|&p| p == pid)
            .ok_or_else(|| InvalidActionError::PlayerNotFound { pid })
    }
}

impl CheckPlayer for Vec<PID> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum Choice {
    Player(PID),
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Kinded)]
enum Phase {
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
        players: Vec<PID>,
        roles: HashMap<PID, Role>,
        contracts: Vec<Contract>,
        phase: Phase,
    },
    End {
        winner: Option<Team>,
        survivors: Vec<PID>,
        contracts: Vec<Contract>,
    },
}

pub struct Game {
    game_id: usize,
    state: State,
    rules: GameRules,
    entrants: Vec<PID>,
    rolegen: RoleGen,
    event_output: EventOutput,
}

impl Game {
    fn handle_vote(
        &mut self,
        voter: PID,
        ballot: Option<Option<PID>>,
    ) -> Result<(), InvalidActionError> {
        // Check valid State
        // Check valid Phase
        // Check valid players

        match &mut self.state {
            State::Play {
                day_num,
                players,
                roles,
                contracts,
                phase,
            } => match phase {
                Phase::Day { votes, blocks } => {
                    if !players.contains(&voter) {
                        return Err(InvalidActionError::PlayerNotFound { pid: voter });
                    }
                }
                _ => {
                    return Err(InvalidActionError::InvalidPhase {
                        expected: PhaseKind::Day,
                        found: phase.kind(),
                    });
                }
            },
            _ => {
                return Err(InvalidActionError::InvalidState {
                    expected: StateKind::Play,
                    found: self.state.kind(),
                });
            }
        }

        if let State::Play {
            day_num,
            players,
            roles,
            contracts,
            phase,
        } = &mut self.state
        {
            if let Phase::Day { votes, blocks } = phase {}
            Ok(())
        } else {
            Err(InvalidActionError::InvalidState)
        }
    }
}
