pub mod player;

pub mod comm;

mod core_test;

use serde::Serialize;
use std::collections::hash_map::{self, Entry, IntoIter, OccupiedEntry};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::fs::File;

use comm::*;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PhaseKind {
    Init,
    Day,
    Night,
    End,
}

impl Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init => write!(f, "Init"),
            Self::Day => write!(f, "Day"),
            Self::Night => write!(f, "Night"),
            Self::End => write!(f, "End"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Day {
    day_no: usize,
    votes: Votes,
    blocked: Vec<Pidx>,
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
// pub struct Night<U: RawPID> {
//     night_no: usize,
//     targets: Targets,
//     scheme: Option<Action<Pidx>>,
//     blocked: Vec<U>,
// }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Target {
    Strip { stripped: Pidx },
    Save { saved: Pidx },
    Investigate { suspect: Pidx },
    Abstain,
}

type Targets = HashMap<Pidx, Target>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Night {
    night_no: usize,
    targets: Targets,
    scheme: Option<(Pidx, Choice<Pidx>)>,
}

impl From<&Night> for Dawn {
    fn from(night: &Night) -> Self {
        // Invert strips...
        let mut block_map = HashMap::new();
        night
            .targets
            .iter()
            .filter_map(|(stripper, t)| match t {
                Target::Strip { stripped } => Some((*stripped, *stripper)),
                _ => None,
            })
            .for_each(|(stripped, stripper)| {
                block_map
                    .entry(stripped)
                    .or_insert_with(Vec::new)
                    .push(stripper);
            });

        // Invert saves...
        let mut save_map = HashMap::new();
        let saves = night
            .targets
            .iter()
            .filter_map(|(doctor, t)| match t {
                Target::Save { saved } => Some((saved, doctor)),
                _ => None,
            })
            .for_each(|(saved, doctor)| {
                save_map
                    .entry(*saved)
                    .or_insert_with(Vec::new)
                    .push(*doctor);
            });
        // Collect Investigations...
        let investigations = night
            .targets
            .iter()
            .filter_map(|(i, t)| match t {
                Target::Investigate { suspect } => Some((*i, *suspect)),
                _ => None,
            })
            .collect();
        // Collect Kill...
        let kill = match night.scheme {
            Some((killer, Choice::Player(mark))) => Some((killer, mark)),
            _ => None,
        };

        Dawn {
            dawn_no: night.night_no + 1,
            block_map,
            save_map,
            investigations,
            kill,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dawn {
    dawn_no: usize,
    block_map: HashMap<Pidx, Vec<Pidx>>,
    save_map: HashMap<Pidx, Vec<Pidx>>,
    investigations: Vec<(Pidx, Pidx)>,
    kill: Option<(Pidx, Pidx)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum Phase {
    Init,
    Day(Day),
    Night(Night),
    End(Winner),
}

impl Phase {
    pub fn clear(&mut self) {
        match self {
            Phase::Day(Day { votes, .. }) => votes.clear(),
            Phase::Night(Night {
                targets, scheme, ..
            }) => {
                targets.clear();
                *scheme = None;
            }
            _ => {}
        }
    }
    pub fn new_day(day_no: usize, blocked: Vec<Pidx>) -> Self {
        Self::Day(Day {
            day_no,
            votes: Vec::new(),
            blocked,
        })
    }
    pub fn new_night(night_no: usize) -> Self {
        Self::Night(Night {
            night_no,
            targets: HashMap::new(),
            scheme: None,
        })
    }
    pub fn kind(&self) -> PhaseKind {
        match self {
            Phase::Init => PhaseKind::Init,
            Phase::Day { .. } => PhaseKind::Day,
            Phase::Night { .. } => PhaseKind::Night,
            Phase::End(_) => PhaseKind::End,
        }
    }
    pub fn is_day<U: RawPID>(&mut self) -> Result<&mut Day, Error<U>> {
        if let Phase::Day(day) = self {
            Ok(day)
        } else {
            Err(Error::InvalidPhase {
                expected: PhaseKind::Day,
                found: self.to_owned(),
            })
        }
    }
    pub fn is_night<U: RawPID>(&mut self) -> Result<&mut Night, Error<U>> {
        if let Phase::Night(night) = self {
            Ok(night)
        } else {
            Err(Error::InvalidPhase {
                expected: PhaseKind::Night,
                found: self.to_owned(),
            })
        }
    }
}
impl Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Init => write!(f, "Init"),
            Phase::Day(Day {
                day_no,
                votes,
                blocked,
            }) => write!(
                f,
                "Day {} (votes: {:?}, blocked: {:?})",
                day_no, votes, blocked
            ),
            Phase::Night(Night {
                night_no,
                targets,
                scheme,
            }) => {
                write!(
                    f,
                    "Night {} (targets: {:?}, scheme: {:?})",
                    night_no, targets, scheme
                )
            }
            Phase::End(winner) => write!(f, "End: {}", winner),
        }
    }
}

fn block_map(targets: &Targets) -> HashMap<Pidx, Vec<Pidx>> {
    let mut block_map = HashMap::new();
    targets
        .iter()
        .filter_map(|(stripper, t)| match t {
            Target::Strip { stripped } => Some((stripped, stripper)),
            _ => None,
        })
        .for_each(|(stripped, stripper)| {
            block_map
                .entry(*stripped)
                .or_insert_with(Vec::new)
                .push(*stripper);
        });
    block_map
}

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

    pub fn get_players_that(
        players: &mut Players<U>,
        f: impl Fn((Pidx, &Player<U>)) -> bool,
    ) -> impl Iterator<Item = (Pidx, &mut Player<U>)> {
        players
            .iter_mut()
            .enumerate()
            .filter(move |(i, p)| f((*i, p)))
    }

    // TODO: Custom error?
    // Handle if directory doesn't exist?
    pub fn save_game(&self, fname: &str) -> Result<(), ()> {
        let mut f = File::create(fname).map_err(|_| ())?;
        serde_json::to_writer_pretty(&mut f, &self).map_err(|_| ())?;
        Ok(())
    }

    // Errors:
    // Invalid phase
    // Can't start game with these roles
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
        if self.check_team_numbers().is_some() {
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
        self.next_phase(next_phase);
        Ok(())
    }

    // fn game_thread(game_mutex: &Mutex<Self>, rx: Receiver<Request<U, S>>) {
    //     loop {
    //         let req = rx.recv().unwrap();
    //         let mut game = game_mutex.lock().unwrap();
    //         if let Err(e) = game.handle(req.cmd) {
    //             // Handle error with context.
    //             // TODO
    //         }
    //     }
    // }

    pub fn handle(&mut self, cmd: Command<U>) -> Result<(), Error<U>> {
        let result = match cmd {
            Command::Vote { voter, ballot } => self.handle_vote(voter, ballot),
            Command::Retract { voter } => self.handle_retract(voter),
            Command::Reveal { celeb } => self.handle_reveal(celeb),
            Command::Target { actor, target } => self.handle_target(actor, target),
            Command::Mark { killer, mark } => self.handle_mark(killer, mark),
        };

        if let SaveStrategy::PerChange(fname) = &self.comm.save {
            self.save_game(fname).expect("Saving game should work");
        };
        result
    }

    fn handle_vote(&mut self, v: U, b: Choice<U>) -> Result<(), Error<U>> {
        self.phase.is_day()?;
        let voter = self.players.check(v)?;
        let ballot = match b {
            Choice::Player(p) => Choice::Player(self.players.check(p)?),
            Choice::Abstain => Choice::Abstain,
        };

        // accept vote?
        self.accept_vote(voter, ballot).map(|(electors, ballot)| {
            self.resolve_election((electors, ballot));
        });
        Ok(())
    }

    fn handle_retract(&mut self, v: U) -> Result<(), Error<U>> {
        self.phase.is_day()?;
        let voter = self.players.check(v)?;

        // How to do this without needing to reproduce day?S
        let day = self.phase.is_day()?;

        let former = day
            .votes
            .iter()
            .position(|(v, _)| v == &voter)
            .map(|i| day.votes.remove(i))
            .map(|(_, b)| b);

        self.comm.tx(Event::Retract {
            voter: self.players[voter].to_owned(),
            former: former.map(|f| f.to_p(&self.players)),
        });

        Ok(())
    }

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
        self.phase.is_night()?;
        let actor = self.players.check(a)?;
        let target = match t {
            Choice::Player(p) => Choice::Player(self.players.check(p)?),
            Choice::Abstain => Choice::Abstain,
        };

        let role = self.players[actor].role;

        self.accept_target(actor, target, role);
        // accept action
        if self.check_dawn().is_some() {
            self.resolve_dawn();
        }
        Ok(())
    }

    fn handle_mark(&mut self, killer: U, mark: Choice<U>) -> Result<(), Error<U>> {
        self.phase.is_night()?;
        let killer = self.players.check(killer)?;
        let mut mark = match mark {
            Choice::Player(p) => Choice::Player(self.players.check(p)?),
            Choice::Abstain => Choice::Abstain,
        };
        let role = self.players[killer].role;
        let night = self.phase.is_night()?;

        // If killer has already targeted tonight, retract that target.
        if let Entry::Occupied(mut e) = night.targets.entry(killer) {
            *e.get_mut() = Target::Abstain;
        }

        match role {
            Role::GOON => {
                mark = Choice::Abstain;
            }
            _ if role.team() == Team::Mafia => {}
            _ => {
                return Err(Error::InvalidRole {
                    role,
                    command: CommandKind::Mark,
                })
            }
        }
        let night = self.phase.is_night()?;
        night.scheme = Some((killer, mark));

        self.comm.tx(Event::Mark {
            killer: self.players[killer].to_owned(),
            mark: mark.to_p(&self.players),
        });

        if self.check_dawn().is_some() {
            self.resolve_dawn();
        }

        Ok(())
    }

    fn accept_vote(&mut self, voter: Pidx, ballot: Choice<Pidx>) -> Option<Election> {
        let day = self.phase.is_day::<U>().expect("Already checked phase");

        let former = day
            .votes
            .iter()
            .position(|(v, _)| v == &voter)
            .map(|i| day.votes.remove(i))
            .map(|(_, b)| b);

        day.votes.push((voter, ballot));

        let n_players = self.players.len();
        let threshold = match ballot {
            Choice::Player(_) => n_players / 2 + 1,
            _ => (n_players + 1) / 2,
        };

        let electors = day
            .votes
            .iter()
            .filter(|(_, b)| b == &ballot)
            .map(|(v, _)| *v)
            .collect::<Vec<_>>();
        let count = electors.len();

        self.comm.tx(Event::Vote {
            voter: self.players[voter].to_owned(),
            ballot: ballot.to_p(&self.players),
            former: former.map(|f| f.to_p(&self.players)),
            count,
            threshold,
        });

        (count >= threshold).then(|| (electors, ballot))
    }

    fn resolve_election(&mut self, (electors, ballot): Election) {
        let day_no = match self.phase {
            Phase::Day(Day { day_no, .. }) => day_no,
            _ => return,
        };

        let hammer = *electors.last().expect("At least one elector");

        let electors = electors
            .into_iter()
            .map(|e| self.players[e].to_owned())
            .collect();

        self.comm.tx(Event::Election {
            electors,
            ballot: ballot.to_p(&self.players),
        });

        if let Choice::Player(elected) = ballot {
            if self.eliminate(&[elected], hammer).is_some() {
                // TODO figure out eliminate returns? As in, what if a team wins?
                return;
            }
        }
        self.next_phase(Phase::new_night(day_no + 1));
    }

    fn accept_target(&mut self, actor: Pidx, target: Choice<Pidx>, role: Role) {
        let night = self.phase.is_night::<U>().expect("Already checked phase");

        // If actor has already targeted tonight, retract that target.
        if let Some((killer, _)) = night.scheme {
            if killer == actor {
                night.scheme = Some((killer, Choice::Abstain));
            }
        }

        self.comm.tx(Event::Target {
            actor: self.players[actor].to_owned(),
            target: target.to_p(&self.players),
        });
        let t = match (role, target) {
            (_, Choice::Abstain) => Target::Abstain,
            (Role::COP, Choice::Player(p)) => Target::Investigate { suspect: p },
            (Role::DOCTOR, Choice::Player(p)) => Target::Save { saved: p },
            (Role::STRIPPER, Choice::Player(p)) => Target::Strip { stripped: p },
            _ => panic!("Shouldn't be able to target with this role"),
        };
        night.targets.insert(actor, t);
    }

    fn check_dawn(&mut self) -> Option<()> {
        let night = self.phase.is_night::<U>().expect("Already checked phase");
        let night_action_count =
            Self::get_players_that(&mut self.players, |(_, p)| p.role.targeting()).count();
        (night_action_count == night.targets.len() && night.scheme.is_some()).then(|| ())
    }

    fn resolve_dawn(&mut self) {
        self.comm.tx(Event::Dawn);

        let night = self.phase.is_night::<U>().expect("Already checked phase");

        let mut block_map = block_map(&night.targets);

        for (actor, target) in &mut night.targets {
            if let Entry::Occupied(e) = block_map.entry(*actor) {
                match target {
                    Target::Save { .. } | Target::Investigate { .. } => {
                        Self::strip_events(&self.comm, e.get(), *actor, &self.players);
                        *target = Target::Abstain;
                    }
                    _ => {}
                }
            }
        }

        // Invert saves...
        let mut save_map = HashMap::new();
        let saves = night
            .targets
            .iter()
            .filter_map(|(doctor, t)| match t {
                Target::Save { saved } => Some((saved, doctor)),
                _ => None,
            })
            .for_each(|(saved, doctor)| {
                save_map
                    .entry(*saved)
                    .or_insert_with(Vec::new)
                    .push(*doctor);
            });
        // Collect Investigations...
        let investigations: Vec<(Pidx, Pidx)> = night
            .targets
            .iter()
            .filter_map(|(i, t)| match t {
                Target::Investigate { suspect } => Some((*i, *suspect)),
                _ => None,
            })
            .collect();

        for (cop, suspect) in investigations {
            let role = self.players[suspect].role;
            self.comm.tx(Event::Investigate {
                cop: self.players[cop].to_owned(),
                suspect: self.players[suspect].to_owned(),
                role,
            });
        }

        match night.scheme {
            Some((killer, Choice::Player(mark))) => {
                if let Entry::Occupied(e) = save_map.entry(mark) {
                    Self::save_events(&self.comm, e.get(), killer, mark, &self.players);
                } else {
                    self.comm.tx(Event::Kill {
                        killer: self.players[killer].to_owned(),
                        mark: self.players[mark].to_owned(),
                    });
                    self.eliminate(&[mark], killer);
                    return;
                }
            }
            _ => {}
        }
        self.comm.tx(Event::NoKill);

        let blocked: Vec<Pidx> = block_map.keys().copied().collect();
        let night_no = night.night_no;
        self.next_phase(Phase::new_day(night_no + 1, blocked));
    }

    // fn resolve_dawn(&mut self) {
    //     self.comm.tx(Event::Dawn);

    //     let night = self.phase.is_night().expect("Already checked phase");

    //     let night_no = night.night_no;

    //     let targets: Vec<(Pidx, Pidx, Role)> = night
    //         .targets
    //         .iter()
    //         .filter_map(|(a, t, r)| t.is_player().map(|p| (*a, p, *r)))
    //         .collect();

    //     // let blocked;
    //     self.resolve_strips();
    //     // (targets, scheme, blocked) = self.resolve_strips(targets, scheme);
    //     self.resolve_saves();
    //     // (targets, scheme) = self.resolve_saves(targets, scheme);

    //     self.resolve_investigations(&targets);

    //     let scheme = self.phase.is_night().unwrap().scheme;
    //     if let Some((killer, Choice::Player(mark))) = scheme {
    //         self.comm.tx(Event::Kill {
    //             killer: self.players[killer].to_owned(),
    //             mark: self.players[mark].to_owned(),
    //         });
    //         if self.eliminate(&[mark], killer).is_some() {
    //             return;
    //         }
    //     } else {
    //         self.comm.tx(Event::NoKill);
    //     }

    //     let blocked = self.phase.is_night().unwrap().blocked.to_owned();

    //     self.next_phase(Phase::new_day(night_no + 1, blocked));
    // }

    // fn resolve_strips(&mut self) {
    //     // collect strips
    //     let mut strips: HashMap<Pidx, Vec<Pidx>> = HashMap::new();
    //     let night = self.phase.is_night().expect("Already checked phase");
    //     for (actor, target, role) in &night.targets {
    //         if *role == Role::STRIPPER {
    //             if let Choice::Player(target) = target {
    //                 strips.entry(*target).or_insert(vec![]).push(*actor);
    //             }
    //         }
    //     }

    //     // let night = self.phase.is_night::<U>().expect("Already checked phase");
    //     night.targets.retain(|(_, _, r)| *r != Role::STRIPPER);
    //     night.targets.retain(|(blocked, t, _)| {
    //         if let Choice::Player(_) = t {
    //             if let Some(strippers) = strips.get(blocked) {
    //                 Self::strip_events(&self.comm, strippers, *blocked, &self.players);
    //                 return false;
    //             }
    //         }
    //         true
    //     });

    //     if let Some((killer, Choice::Player(_))) = night.scheme {
    //         if let Some(strippers) = strips.get(&killer) {
    //             Self::strip_events(&self.comm, strippers, killer, &self.players);
    //             night.scheme = Some((killer, Choice::Abstain));
    //         }
    //     }

    //     night.blocked = strips.keys().map(|p| self.players[*p].raw_pid).collect();
    // }

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

    // fn resolve_saves(&mut self) {
    //     // collect saves?
    //     let mut saves: HashMap<Pidx, Vec<Pidx>> = HashMap::new();
    //     let night = self.phase.is_night().expect("Already checked phase");
    //     for (actor, target, role) in &night.targets {
    //         if *role == Role::DOCTOR {
    //             if let Choice::Player(target) = target {
    //                 saves.entry(*target).or_insert(vec![]).push(*actor);
    //             }
    //         }
    //     }

    //     if let Some((killer, Choice::Player(mark))) = night.scheme {
    //         if let Some(doctors) = saves.get(&mark) {
    //             Self::save_events(&self.comm, doctors, killer, mark, &self.players);
    //             night.scheme = Some((killer, Choice::Abstain));
    //         }
    //     }
    // }

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

    // fn resolve_investigations(&mut self, targets: &Vec<(Pidx, Pidx, Role)>) {
    //     for (cop, suspect, role) in targets {
    //         let (cop, suspect) = (*cop, *suspect);
    //         if *role == Role::COP {
    //             let role = self.players[suspect].role;
    //             let (cop, suspect) = (
    //                 self.players[cop].to_owned(),
    //                 self.players[suspect].to_owned(),
    //             );
    //             self.comm.tx(Event::Investigate { cop, suspect, role });
    //         }
    //     }
    // }

    fn eliminate(&mut self, players: &[Pidx], _: Pidx) -> Option<Winner> {
        let mut players = players.to_owned();
        players.sort();
        // Remove from largest to smallest to avoid invalidating indices
        for player in players.into_iter().rev() {
            let player_ = self.players[player].to_owned();
            self.comm.tx(Event::Eliminate { player: player_ });

            self.players.remove(player);
        }
        // all Pidxs are now invalid...
        self.phase.clear();

        let winner = self.check_team_numbers();
        if let Some(winner) = winner {
            self.next_phase(Phase::End(winner));
        }
        winner
    }

    fn next_phase(&mut self, next_phase: Phase) {
        self.phase = next_phase;

        match self.phase {
            Phase::Day(Day { day_no, .. }) => self.comm.tx(Event::Day { day_no }),
            Phase::Night(Night { night_no, .. }) => self.comm.tx(Event::Night { night_no }),
            Phase::End(winner) => self.comm.tx(Event::End { winner }),
            _ => panic!("Should never go to Init Phase!"),
        }

        if let SaveStrategy::PerPhase(fname) = &self.comm.save {
            self.save_game(fname).expect("Saving game should work");
        };
    }

    fn check_team_numbers(&self) -> Option<Winner> {
        let n_players = self.players.len();
        let n_mafia = self
            .players
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
}
