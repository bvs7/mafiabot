pub mod player;

pub mod comm;

mod test;

use serde::Serialize;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

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
    votes: Vec<Action<Pidx>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Night {
    night_no: usize,
    targets: Vec<Action<Pidx>>,
    scheme: Option<Action<Pidx>>,
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
        match &self {
            Phase::Day(Day { day_no, .. }) => *self = Phase::new_day(*day_no),
            Phase::Night(Night { night_no, .. }) => *self = Phase::new_night(*night_no),
            _ => {}
        }
    }
    pub fn new_day(day_no: usize) -> Self {
        Self::Day(Day {
            day_no,
            votes: Vec::new(),
        })
    }
    pub fn new_night(night_no: usize) -> Self {
        Self::Night(Night {
            night_no,
            targets: Vec::new(),
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
            Phase::Day(Day { day_no, votes }) => write!(f, "Day {} (votes: {:?})", day_no, votes),
            Phase::Night(Night {
                night_no,
                targets,
                scheme,
            }) => {
                write!(
                    f,
                    "Night {} (targets: {:?}, scheme: {:?}",
                    night_no, targets, scheme
                )
            }
            Phase::End(winner) => write!(f, "End: {}", winner),
        }
    }
}

// Want to ensure players can't be modified without clearing phase...
type Players<U> = Vec<Player<U>>;

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

        for player in players {
            if game.check_player(player.raw_pid).is_err() {
                game.players.push(player);
            }
        }

        game
    }

    pub fn check_player(&self, raw_pid: U) -> Result<Pidx, Error<U>> {
        self.players
            .iter()
            .position(|p| p.raw_pid == raw_pid)
            .ok_or_else(|| Error::PlayerNotFound { pid: raw_pid })
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
            false => Phase::new_day(1),
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

    fn handle_vote(&mut self, v: U, b: Target<U>) -> Result<(), Error<U>> {
        self.phase.is_day()?;
        let voter = self.check_player(v)?;
        let ballot = match b {
            Target::Player(p) => Target::Player(self.check_player(p)?),
            Target::Abstain => Target::Abstain,
            Target::Blocked => Target::Blocked, // TODO: Err?
        };

        // accept vote?
        self.accept_vote(voter, ballot).map(|(electors, ballot)| {
            self.resolve_election((electors, ballot));
        });
        Ok(())
    }

    fn handle_retract(&mut self, v: U) -> Result<(), Error<U>> {
        self.phase.is_day()?;
        let voter = self.check_player(v)?;

        let day = self.phase.is_day::<U>().expect("Already checked");

        let former = day
            .votes
            .iter()
            .position(|(v, _)| v == &voter)
            .map(|i| day.votes.remove(i))
            .map(|(_, b)| b);

        self.comm.tx(Event::Retract { voter, former });

        Ok(())
    }

    fn handle_reveal(&mut self, celeb: U) -> Result<(), Error<U>> {
        self.phase.is_day()?;
        let celeb = self.check_player(celeb)?;
        if self.players[celeb].role != Role::CELEB {
            return Err(Error::InvalidRole {
                role: self.players[celeb].role,
                command: CommandKind::Reveal,
            });
        }
        self.comm.tx(Event::Reveal { celeb });
        Ok(())
    }

    fn handle_target(&mut self, a: U, t: Target<U>) -> Result<(), Error<U>> {
        self.phase.is_night()?;
        let actor = self.check_player(a)?;
        let target = match t {
            Target::Player(p) => Target::Player(self.check_player(p)?),
            Target::Abstain => Target::Abstain,
            Target::Blocked => Target::Blocked, // TODO: Err?
        };

        // accept action
        if self.accept_target(actor, target).is_some() {
            self.resolve_dawn();
        }
        Ok(())
    }

    fn handle_mark(&mut self, killer: U, mark: Target<U>) -> Result<(), Error<U>> {
        self.phase.is_night()?;
        let killer = self.check_player(killer)?;
        let mark = match mark {
            Target::Player(p) => Target::Player(self.check_player(p)?),
            Target::Abstain => Target::Abstain,
            Target::Blocked => Target::Blocked, // TODO: Err?
        };
        if self.players[killer].role.team() != Team::Mafia {
            return Err(Error::InvalidRole {
                role: self.players[killer].role,
                command: CommandKind::Mark,
            });
        }
        let night = self.phase.is_night::<U>().expect("Already checked");
        night.scheme = Some((killer, mark));
        Ok(())
    }

    fn accept_vote(&mut self, voter: Pidx, ballot: Target<Pidx>) -> Option<Election> {
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
            Target::Player(_) => n_players / 2 + 1,
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
            voter,
            ballot,
            former,
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

        self.comm.tx(Event::Election {
            electors: electors,
            ballot,
        });

        if let Target::Player(elected) = ballot {
            if self.eliminate(elected, hammer).is_some() {
                // TODO figure out eliminate returns? As in, what if a team wins?
                return;
            }
        }
        self.next_phase(Phase::new_night(day_no + 1));
    }

    fn accept_target(&mut self, actor: Pidx, target: Target<Pidx>) -> Option<()> {
        // Check for Goon
        let target = match self.players[actor].role {
            Role::GOON => Target::Abstain,
            _ => target,
        };

        let night = self.phase.is_night::<U>().expect("Already checked phase");
        self.comm.tx(Event::Target { actor, target });
        night.targets.push((actor, target));

        let night_action_count =
            Self::get_players_that(&mut self.players, |(i, p)| p.role.targeting()).count();

        (night_action_count == night.targets.len() && night.scheme.is_some()).then(|| ())
    }

    fn resolve_dawn(&mut self) {
        self.comm.tx(Event::Dawn);
        // Strip
        let night = self.phase.is_night::<U>().expect("Already checked phase");

        // Handle Strips
        for (stripper, target) in night.targets {
            if self.players[stripper].role == Role::STRIPPER {
                Self::strip(&mut night.targets, *stripper, *target);
            }
        }

        // Handle Saves
        // night
        //     .targets
        //     .iter()
        //     .filter(|(actor, _)| self.players[*actor].role == Role::DOCTOR)
        //     .for_each(|(doctor, target)| self.save(*doctor, *target));

        // // Handle Investigations
        // night
        //     .targets
        //     .iter()
        //     .filter(|(actor, _)| self.players[*actor].role == Role::COP)
        //     .for_each(|(cop, target)| self.investigate(*cop, *target));

        // if let Some((killer, Target::Player(mark))) = night.scheme {
        //     // (Copy to avoid borrow checker)
        //     self.comm.tx(Event::Kill { killer, mark });
        //     if self.eliminate(mark, killer).is_some() {
        //         return;
        //     }
        // } else {
        //     self.comm.tx(Event::NoKill);
        // }
        // self.next_phase(Phase::new_day(night.night_no + 1));
    }

    fn strip(&mut targets, stripper: Pidx, stripped: Target<Pidx>) {
        let stripped = match stripped {
            Target::Player(p) if self.players[p].role != Role::STRIPPER => p,
            _ => return,
        };

        // Handle the case where stripper blocks mafia killer
        match night.scheme {
            Some((actor, mut target)) if actor == stripped => {
                self.comm.tx(Event::Strip { stripper, stripped });
                match target {
                    Target::Player(_) | Target::Abstain => {
                        self.comm.tx(Event::Block { blocked: stripped });
                    }
                    Target::Blocked => {}
                }
                target = Target::Blocked;
                return;
            }
            _ => {}
        }

        let former = night
            .targets
            .iter()
            .position(|(a, t)| *a == stripped)
            .map(|i| night.targets.remove(i));

        if let Some(action) = former {
            self.comm.tx(Event::Strip { stripper, stripped });
            match action {
                (stripped, Target::Player(_)) | (stripped, Target::Abstain) => {
                    self.comm.tx(Event::Block { blocked: stripped });
                }
                (stripped, Target::Blocked) => {}
            }
            night.targets.push((stripped, Target::Blocked));
        }
    }

    fn save(&mut self, doctor: Pidx, patient: Target<Pidx>) {
        let night = self.phase.is_night::<U>().expect("Already checked phase");
        let saved = match patient {
            Target::Player(p) => p,
            _ => return,
        };
        if let Some((_, mut target)) = night.scheme {
            if let Target::Player(mark) = target {
                if mark == saved {
                    self.comm.tx(Event::Save { doctor, saved });
                    target = Target::Blocked;
                }
            }
        }
    }

    fn investigate(&mut self, cop: Pidx, suspect: Target<Pidx>) {
        let suspect = match suspect {
            Target::Player(p) => p,
            _ => return,
        };
        let role = self.players[suspect].role;
        self.comm.tx(Event::Investigate { cop, suspect, role });
    }

    fn eliminate(&mut self, player: Pidx, proxy: Pidx) -> Option<Winner> {
        self.comm.tx(Event::Eliminate { player: player });
        self.players.remove(player);
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
