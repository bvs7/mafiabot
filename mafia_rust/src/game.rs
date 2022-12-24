pub mod comm;
pub mod phase;
pub mod player;

mod core_test;

use serde::Serialize;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::fs::File;

use comm::*;
use phase::*;
use player::*;

#[derive(Debug)]
pub enum Error<U: RawPID> {
    InvalidPhase {
        expected: PhaseKind,
        found: Phase,
    },
    InvalidCommand {
        command: CommandKind,
        phase: PhaseKind,
    },
    PlayerNotFound {
        pid: U,
    },
    InvalidRole {
        role: Role,
        command: CommandKind,
    },
}

impl<U: RawPID> Display for Error<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error with command: ")?;
        match self {
            Self::InvalidPhase { expected, found } => {
                write!(
                    f,
                    "Invalid Phase (expected {:?}, found {:?}",
                    expected, found
                )
            }
            Self::InvalidCommand { command, phase } => {
                write!(f, "Invalid Command ({:?}) for Phase ({:?})", command, phase)
            }
            Self::PlayerNotFound { pid } => {
                write!(f, "Player with UserID {:?} not found", pid)
            }
            Self::InvalidRole { role, command } => {
                write!(f, "Invalid Role ({:?}) for Command ({:?})", role, command)
            }
        }
    }
}
impl<U: RawPID> std::error::Error for Error<U> {}

pub trait PlayerCheck<U: RawPID> {
    fn check(&self, raw_pid: U) -> Result<Pidx, Error<U>>;
}

// Want to ensure players can't be modified without clearing phase...
type Players<U> = Vec<Player<U>>;

impl<U: RawPID> PlayerCheck<U> for Players<U> {
    fn check(&self, raw_pid: U) -> Result<Pidx, Error<U>> {
        self.iter()
            .position(|p| p.raw_pid == raw_pid)
            .ok_or_else(|| Error::PlayerNotFound { pid: raw_pid })
    }
}

#[derive(Debug, Serialize /*Deserialize*/)]
pub struct Game<U: RawPID, S: Source> {
    players: Players<U>,
    phase: Phase,
    #[serde(skip)]
    comm: Comm<U, S>,
}

impl<U: RawPID, S: Source> Game<U, S> {
    pub fn new(players: Players<U>, comm: Comm<U, S>) -> Self {
        let mut game = Self {
            players: Vec::new(),
            phase: Phase::Init,
            comm,
        };

        game.comm.tx(Event::Init);

        for player in &players {
            if players.check(player.raw_pid).is_err() {
                game.players.push(*player);
            }
        }

        game
    }
}

pub fn get_players_that<U: RawPID>(
    players: &Players<U>,
    f: impl Fn((Pidx, &Player<U>)) -> bool,
) -> impl Iterator<Item = (Pidx, &Player<U>)> {
    players.iter().enumerate().filter(move |(i, p)| f((*i, p)))
}
impl<U: RawPID, S: Source> Game<U, S> {
    // TODO: Custom error?
    // Handle if directory doesn't exist?
    pub fn save_game(&self, fname: &str) -> Result<(), ()> {
        let mut f = File::create(fname).map_err(|_| ())?;
        serde_json::to_writer_pretty(&mut f, &self).map_err(|_| ())?;
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), ()> {
        match self.phase {
            Phase::Init => {}
            _ => return Err(()),
        }
        if self.players.len() < 3 {
            // self.comm.tx(Event::InvalidCommand(
            //     "Can't start game with less than 3 players".to_string(),
            // ));
            return Err(());
        }
        if check_team_numbers(&self.players).is_some() {
            // self.comm.tx(Event::InvalidCommand(
            //     "Can't start game with given roles".to_string(),
            // ));
            return Err(());
        }
        let next_phase = match self.players.len() % 2 == 0 {
            true => Phase::new_night(1),
            false => Phase::new_day(1, Vec::new()),
        };
        self.comm.tx(Event::Start {
            players: self.players.clone(),
            phase: next_phase.kind(),
        });
        self.phase.next_phase(next_phase, &self.comm);
        Ok(())
    }

    pub fn handle(&mut self, cmd: Command<U>) -> Result<(), Error<U>> {
        let result = match cmd {
            Command::Vote { voter, ballot } => self.handle_vote(voter, ballot),
            Command::Reveal { celeb } => self.handle_reveal(celeb),
            Command::Target { actor, target } => self.handle_target(actor, target),
            Command::Mark { killer, mark } => self.handle_mark(killer, mark),
        };

        if let SaveStrategy::PerChange(fname) = &self.comm.save {
            self.save_game(fname).expect("Saving game should work");
        };
        result
    }

    fn handle_vote(&mut self, v: U, b: Option<Choice<U>>) -> Result<(), Error<U>> {
        let day = self.phase.is_day()?;
        let voter = self.players.check(v)?;
        let ballot = match b {
            Some(Choice::Player(p)) => Some(Choice::Player(self.players.check(p)?)),
            Some(Choice::Abstain) => Some(Choice::Abstain),
            None => None,
        };

        // accept vote?
        let day_resolution = day.resolve_vote(&self.players, (voter, ballot), &self.comm);

        let next_phase: Phase = match day_resolution {
            Some(DayResolution::Elected(elected, electors, hammer, next_phase)) => self
                .phase
                .eliminate(&mut self.players, &[elected], hammer, &self.comm)
                .unwrap_or(next_phase),
            Some(DayResolution::NoKill(next_phase)) => next_phase,
            None => return Ok(()),
        };

        self.phase.next_phase(next_phase, &self.comm);
        Ok(())
    }

    // fn handle_retract(&mut self, v: U) -> Result<(), Error<U>> {
    //     self.phase.is_day()?;
    //     let voter = self.players.check(v)?;

    //     // How to do this without needing to reproduce day?S
    //     let day = self.phase.is_day()?;

    //     let former = day
    //         .votes
    //         .iter()
    //         .position(|(v, _)| v == &voter)
    //         .map(|i| day.votes.remove(i))
    //         .map(|(_, b)| b);

    //     self.comm.tx(Event::Retract {
    //         voter: self.players[voter].to_owned(),
    //         former: former.map(|f| f.to_p(&self.players)),
    //     });

    //     Ok(())
    // }

    fn handle_reveal(&mut self, celeb: U) -> Result<(), Error<U>> {
        // Can we do this with a single check of day? TODO

        let day = self.phase.is_day()?;
        let celeb = self.players.check(celeb)?;
        if self.players[celeb].role != Role::CELEB {
            return Err(Error::InvalidRole {
                role: self.players[celeb].role,
                command: CommandKind::Reveal,
            });
        }

        if day.blocked.contains(&celeb) {
            self.comm.tx(Event::Block {
                blocked: self.players[celeb].to_owned(),
            });
            return Ok(());
        }
        self.comm.tx(Event::Reveal {
            celeb: self.players[celeb].to_owned(),
        });
        Ok(())
    }

    fn handle_target(&mut self, a: U, t: Choice<U>) -> Result<(), Error<U>> {
        let night = self.phase.is_night()?;
        let actor = self.players.check(a)?;
        let target = match t {
            Choice::Player(p) => Choice::Player(self.players.check(p)?),
            Choice::Abstain => Choice::Abstain,
        };

        let role = self.players[actor].role;

        let night_resolution = night.resolve_target(&self.players, actor, target, role, &self.comm);

        self.handle_dawn(night_resolution);

        Ok(())
    }

    fn handle_mark(&mut self, killer: U, mark: Choice<U>) -> Result<(), Error<U>> {
        let night = self.phase.is_night()?;
        let killer = self.players.check(killer)?;
        let mut mark = match mark {
            Choice::Player(p) => Choice::Player(self.players.check(p)?),
            Choice::Abstain => Choice::Abstain,
        };
        let role = self.players[killer].role;

        match role {
            Role::GOON => {
                mark = Choice::Abstain;
            }
            _ if role.team() == Team::Mafia => {}
            _ => {
                return Err(Error::InvalidRole {
                    role,
                    command: CommandKind::Mark,
                });
            }
        };

        let night_resolution = night.resolve_mark(&self.players, killer, mark, &self.comm);

        self.handle_dawn(night_resolution);

        Ok(())
    }

    fn handle_dawn(&mut self, night_resolution: Option<NightResolution>) {
        let phase = match night_resolution {
            Some(NightResolution::Kill(killer, mark, next_phase)) => self
                .phase
                .eliminate(&mut self.players, &[mark], killer, &self.comm)
                .unwrap_or(next_phase),
            Some(NightResolution::NoKill(next_phase)) => next_phase,
            None => return,
        };

        self.phase.next_phase(phase, &self.comm);
    }

    fn strip_events(
        comm: &Comm<U, S>,
        strippers: &Vec<Pidx>,
        blocked: Pidx,
        players: &Vec<Player<U>>,
    ) {
        comm.tx(Event::Block {
            blocked: players[blocked].to_owned(),
        });
        for stripper in strippers {
            comm.tx(Event::Strip {
                stripper: players[*stripper].to_owned(),
                blocked: players[blocked].to_owned(),
            });
        }
    }

    fn save_events(
        comm: &Comm<U, S>,
        doctors: &Vec<Pidx>,
        killer: Pidx,
        saved: Pidx,
        players: &Vec<Player<U>>,
    ) {
        comm.tx(Event::Block {
            blocked: players[killer].to_owned(),
        });
        for doctor in doctors {
            comm.tx(Event::Save {
                doctor: players[*doctor].to_owned(),
                saved: players[saved].to_owned(),
            });
        }
    }
}

fn check_team_numbers<U: RawPID>(players: &Players<U>) -> Option<Winner> {
    let n_players = players.len();
    let n_mafia = players
        .iter()
        .filter(|p| p.role.team() == Team::Mafia)
        .count();

    match 0 {
        _ if n_mafia == 0 => Some(Winner::Team(Team::Town)),
        // True if: 3/5, 3/6. False if: 2/5, 2/6
        _ if n_mafia > (n_players - 1) / 2 => Some(Winner::Team(Team::Mafia)),
        _ => None,
    }
}
