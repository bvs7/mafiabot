pub mod comm;
pub mod phase;
pub mod player;

mod core_test;

use serde::Serialize;
use std::fmt::{Debug, Display};
use std::fs::File;

use comm::*;
use phase::*;
use player::*;

#[derive(Debug)]
pub enum Error<U: RawPID> {
    InvalidPhase {
        expected: PhaseKind,
        found: Phase<U>,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum ChargeStatus {
    #[default]
    Alive,
    Dead,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum IdiotStatus {
    #[default]
    Unelected,
    Elected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Contract<U: RawPID> {
    Protect {
        holder: U,
        charge: U,
        status: ChargeStatus,
    },
    Assassinate {
        holder: U,
        charge: U,
        status: ChargeStatus,
    },
    Elect {
        holder: U,
        status: IdiotStatus,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum ContractResult<U: RawPID> {
    Win { holder: U },
    Loss { holder: U },
}

impl<U: RawPID> Contract<U> {
    pub fn refocus<S: Source>(
        &mut self,
        players: &Players<U>,
        died: U,
        proxy: U,
        comm: &Comm<U, S>,
    ) {
        match self {
            Contract::Assassinate {
                holder,
                charge,
                status,
            }
            | Contract::Protect {
                holder,
                charge,
                status,
            } if *charge == died => {
                // Check if alive
                if players.check(*holder).is_ok() {
                    let new_charge = if players.check(proxy).is_ok() {
                        proxy
                    } else {
                        *holder
                    };
                    *self = self.to_refocused(new_charge);
                    comm.tx(Event::Refocus {
                        new_contract: *self,
                    })
                } else {
                    *status = ChargeStatus::Dead;
                }
            }
            _ => {}
        }
    }

    pub fn to_refocused(&self, charge: U) -> Self {
        match self {
            Contract::Assassinate { holder, .. } => Contract::Protect {
                holder: *holder,
                charge,
                status: ChargeStatus::Alive,
            },
            Contract::Protect { holder, .. } => Contract::Assassinate {
                holder: *holder,
                charge,
                status: ChargeStatus::Alive,
            },
            _ => panic!("Should not refocus this contract"),
        }
    }

    fn check_win(&self) -> ContractResult<U> {
        match self {
            Contract::Assassinate { holder, status, .. } if *status == ChargeStatus::Dead => {
                ContractResult::Win { holder: *holder }
            }
            Contract::Protect { holder, status, .. } if *status == ChargeStatus::Alive => {
                ContractResult::Win { holder: *holder }
            }
            Contract::Elect { holder, status } if *status == IdiotStatus::Elected => {
                ContractResult::Win { holder: *holder }
            }
            Contract::Assassinate { holder, .. }
            | Contract::Protect { holder, .. }
            | Contract::Elect { holder, .. } => ContractResult::Loss { holder: *holder },
        }
    }
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
    phase: Phase<U>,
    contracts: Vec<Contract<U>>,
    #[serde(skip)]
    comm: Comm<U, S>,
}

impl<U: RawPID, S: Source> Game<U, S> {
    pub fn new(players: Players<U>, comm: Comm<U, S>) -> Self {
        let mut game = Self {
            players: Vec::new(),
            phase: Phase::Init,
            contracts: Vec::new(),
            comm,
        };

        game.comm.tx(Event::Init);

        for player in &players {
            if players.check(player.raw_pid).is_err() {
                game.players.push(player.to_owned());
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

        let next_phase: Phase<U> = match day_resolution {
            Some(DayResolution::Elected(elected, _electors, hammer, next_phase)) => {
                self.check_elect_contract(elected);
                self.eliminate(&[elected], hammer).unwrap_or(next_phase)
            }
            Some(DayResolution::NoKill(next_phase)) => next_phase,
            None => return Ok(()),
        };

        self.phase.next_phase(next_phase, &self.comm);
        Ok(())
    }

    fn check_elect_contract(&mut self, elected: Pidx) {
        let elected_id = self.players[elected].raw_pid;
        for contract in self.contracts.iter_mut() {
            if let Contract::Elect { holder, status } = contract {
                if *holder == elected_id {
                    *status = IdiotStatus::Elected;
                }
            }
        }
    }

    fn handle_reveal(&mut self, celeb: U) -> Result<(), Error<U>> {
        let day = self.phase.is_day()?;
        let celeb = self.players.check(celeb)?;
        if self.players[celeb].role != Role::CELEB {
            return Err(Error::InvalidRole {
                role: self.players[celeb].role.to_owned(),
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

        let role = self.players[actor].role.to_owned();

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
        let role = self.players[killer].role.to_owned();

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

    fn handle_dawn(&mut self, night_resolution: Option<NightResolution<U>>) {
        let next_phase = match night_resolution {
            Some(NightResolution::Kill(killer, mark, phase)) => {
                self.eliminate(&[mark], killer).unwrap_or(phase)
            }
            Some(NightResolution::NoKill(phase)) => phase,
            None => return,
        };

        self.phase.next_phase(next_phase, &self.comm);
    }

    pub fn eliminate(&mut self, to_die: &[Pidx], proxy: Pidx) -> Option<Phase<U>> {
        let mut to_die = to_die.to_owned();
        to_die.sort();

        let mut to_die_ids = Vec::<U>::new();
        let proxy_id = self.players[proxy].raw_pid;

        // Remove from largest to smallest to avoid invalidating indices
        for p in to_die.into_iter().rev() {
            let player = self.players[p].to_owned();
            to_die_ids.push(player.raw_pid);
            self.comm.tx(Event::Eliminate { player });

            self.players.remove(p);
        }
        // all Pidxs are now invalid...
        self.phase.clear();

        // Check contracts
        for p_id in to_die_ids {
            self.check_contracts(p_id, proxy_id)
        }

        let winner = check_team_numbers(&self.players);

        if let Some(win) = winner {
            let contract_results: Vec<_> = self.contracts.iter().map(|c| c.check_win()).collect();
            return Some(Phase::End(win, contract_results));
        }
        None
    }

    fn check_contracts(&mut self, died: U, proxy: U) {
        for contract in self.contracts.iter_mut() {
            contract.refocus(&self.players, died, proxy, &self.comm);
        }
    }
}

fn check_team_numbers<U: RawPID>(players: &Players<U>) -> Option<Winner> {
    let n_players = players.len();
    let n_mafia = players
        .iter()
        .filter(|p| p.role.team() == Team::Mafia)
        .count();

    if n_mafia == 0 {
        Some(Winner::Team(Team::Town))
    } else if n_mafia > (n_players - 1) / 2 {
        Some(Winner::Team(Team::Mafia))
    } else {
        None
    }
}

mod test {
    use super::comm::DisplayEventHandler;
    use super::*;
    use std::sync::mpsc::Receiver;
    use std::thread;
    use std::time::Duration;

    impl RawPID for u64 {}
    impl Source for String {}

    fn _basics() -> (
        Comm<u64, String>,
        Receiver<Response<u64, String>>,
        DisplayEventHandler,
    ) {
        let (_, rx) = std::sync::mpsc::channel();
        let (tx, rx_out) = std::sync::mpsc::channel();

        let comm: Comm<u64, String> = Comm::new(rx, tx);
        let deh = DisplayEventHandler::new();
        return (comm, rx_out, deh);
    }

    #[allow(dead_code)]
    fn resp_handle(
        rx: &mut Receiver<Response<u64, String>>,
        eh: &mut impl EventHandler<u64, String>,
    ) {
        loop {
            thread::sleep(Duration::from_millis(50));
            match rx.try_recv() {
                Ok(resp) => eh.handle(resp.event, resp.src),
                Err(_) => break,
            }
        }
    }
    #[test]
    fn test_refocus() {
        let players = vec![
            Player {
                raw_pid: 1u64,
                role: Role::TOWN,
            },
            Player {
                raw_pid: 2u64,
                role: Role::GUARD,
            },
            Player {
                raw_pid: 3u64,
                role: Role::GUARD,
            },
            Player {
                raw_pid: 4u64,
                role: Role::AGENT,
            },
            Player {
                raw_pid: 5u64,
                role: Role::AGENT,
            },
            Player {
                raw_pid: 6u64,
                role: Role::IDIOT,
            },
            Player {
                raw_pid: 7u64,
                role: Role::IDIOT,
            },
            Player {
                raw_pid: 8u64,
                role: Role::MAFIA,
            },
            Player {
                raw_pid: 9u64,
                role: Role::MAFIA,
            },
        ];

        let mut contracts = vec![
            Contract::Protect {
                holder: 2u64,
                charge: 1,
                status: ChargeStatus::Alive,
            },
            Contract::Protect {
                holder: 3,
                charge: 8,
                status: ChargeStatus::Alive,
            },
            Contract::Assassinate {
                holder: 4,
                charge: 1,
                status: ChargeStatus::Alive,
            },
            Contract::Assassinate {
                holder: 5,
                charge: 8,
                status: ChargeStatus::Alive,
            },
            Contract::Elect {
                holder: 6,
                status: IdiotStatus::Unelected,
            },
            Contract::Elect {
                holder: 7,
                status: IdiotStatus::Unelected,
            },
        ];

        let (comm, mut _rx, mut _deh) = _basics();

        let mut game = Game {
            players,
            phase: Phase::new_night(1),
            contracts: contracts.clone(),
            comm,
        };

        assert_eq!(game.contracts, contracts);

        assert!(game.handle_mark(8, Choice::Player(1)).is_ok());

        assert!(matches!(game.phase, Phase::Day(_)));

        contracts[0] = Contract::Assassinate {
            holder: 2,
            charge: 8,
            status: ChargeStatus::Alive,
        };
        contracts[2] = Contract::Protect {
            holder: 4,
            charge: 8,
            status: ChargeStatus::Alive,
        };

        assert_eq!(game.contracts, contracts);

        game.phase = Phase::new_night(2);
        assert!(game.handle_mark(9, Choice::Player(2)).is_ok());
        game.phase = Phase::new_night(3);
        assert!(game.handle_mark(9, Choice::Player(4)).is_ok());

        game.phase = Phase::new_night(4);
        assert!(game.handle_mark(9, Choice::Player(8)).is_ok());

        contracts[0] = Contract::Assassinate {
            holder: 2,
            charge: 8,
            status: ChargeStatus::Dead,
        };
        contracts[1] = Contract::Assassinate {
            holder: 3,
            charge: 9,
            status: ChargeStatus::Alive,
        };
        contracts[2] = Contract::Protect {
            holder: 4,
            charge: 8,
            status: ChargeStatus::Dead,
        };
        contracts[3] = Contract::Protect {
            holder: 5,
            charge: 9,
            status: ChargeStatus::Alive,
        };

        // 3,5,6,7,9

        assert_eq!(game.contracts, contracts);

        assert!(game.handle_vote(3, Some(Choice::Player(6))).is_ok());
        assert!(game.handle_vote(5, Some(Choice::Player(6))).is_ok());
        assert!(game.handle_vote(6, Some(Choice::Player(6))).is_ok());

        contracts[4] = Contract::Elect {
            holder: 6,
            status: IdiotStatus::Elected,
        };

        assert_eq!(game.contracts, contracts);

        assert!(game.handle_mark(9, Choice::Player(9)).is_ok());

        assert_eq!(
            game.phase,
            Phase::End(
                Winner::Team(Team::Town),
                vec![
                    ContractResult::Win { holder: 2 },
                    ContractResult::Win { holder: 3 },
                    ContractResult::Loss { holder: 4 },
                    ContractResult::Loss { holder: 5 },
                    ContractResult::Win { holder: 6 },
                    ContractResult::Loss { holder: 7 },
                ]
            )
        );
    }
}
