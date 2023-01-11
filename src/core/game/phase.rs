use serde::Serialize;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Ballot {
    Player(Pidx),
    Abstain,
}

impl Ballot {
    fn to_p<U: RawPID>(&self, players: &Players<U>) -> Option<Player<U>> {
        match self {
            Ballot::Player(p) => Some(players[*p].clone()),
            Ballot::Abstain => None,
        }
    }
}

pub type Vote = (Pidx, Ballot);
pub type Votes = Vec<Vote>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Target {
    Strip(Pidx),
    Save(Pidx),
    Investigate(Pidx),
    Abstain,
}
pub type Targets = HashMap<Pidx, Target>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Mark {
    Kill(Pidx, Pidx),
    Abstain,
}

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

pub enum DayResolution<U: RawPID> {
    Elected(Pidx, Vec<Pidx>, Pidx, Phase<U>),
    NoKill(Phase<U>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Day {
    pub day_no: usize,
    pub votes: Votes,
    pub blocked: Vec<Pidx>,
}

impl Day {
    pub fn resolve_vote<U: RawPID>(
        &mut self,
        players: &Vec<Player<U>>,
        voter: Pidx,
        choice: Option<Ballot>,
        comm: &Comm<U>,
    ) -> Option<DayResolution<U>> {
        let former = self
            .votes
            .iter()
            .position(|(v, _)| v == &voter)
            .map(|i| self.votes.remove(i))
            .map(|(_, b)| b);

        let ballot = match choice {
            Some(b) => {
                self.votes.push((voter, b.clone()));
                b
            }
            None => {
                comm.tx(Event::Retract {
                    voter: players[voter].to_owned(),
                    former: former.map(|b| b.to_p(players)),
                });
                return None; // Vote retraction can't cause election
            }
        };

        let n_players = players.len();
        let threshold = match ballot {
            Ballot::Player(_) => n_players / 2 + 1,
            _ => (n_players + 1) / 2,
        };

        let electors = self
            .votes
            .iter()
            .filter(|(_, b)| b == &ballot)
            .map(|(v, _)| *v)
            .collect::<Vec<_>>();
        let count = electors.len();

        comm.tx(Event::Vote {
            voter: players[voter].to_owned(),
            ballot: ballot.to_p(players),
            former: former.map(|f| f.to_p(&players)),
            count,
            threshold,
        });

        if count < threshold {
            return None;
        }
        // Election has occured!
        let &hammer = electors.last().expect("At least one elector");

        let electors_p: Vec<Player<U>> = electors.iter().map(|e| players[*e].to_owned()).collect();

        comm.tx(Event::Election {
            electors: electors_p,
            ballot: ballot.to_p(&players),
        });

        let next_phase = Phase::new_night(self.day_no);
        if let Ballot::Player(elected) = ballot {
            Some(DayResolution::Elected(
                elected, electors, hammer, next_phase,
            ))
        } else {
            Some(DayResolution::NoKill(next_phase))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Night {
    night_no: usize,
    pub targets: Targets,
    pub scheme: Option<Mark>,
}

pub enum NightResolution<U: RawPID> {
    NoKill(Phase<U>),
    Kill(Pidx, Pidx, Phase<U>),
}

impl Night {
    pub fn resolve_target<U: RawPID>(
        &mut self,
        players: &Vec<Player<U>>,
        actor: Pidx,
        choice: Choice<Pidx>,
        role: Role,
        comm: &Comm<U>,
    ) -> Option<NightResolution<U>> {
        // If actor has already targeted tonight, retract that target.
        if let Some(Mark::Kill(killer, _)) = self.scheme {
            if killer == actor {
                self.scheme = Some(Mark::Abstain);
            }
        }
        comm.tx(Event::Target {
            actor: players[actor].to_owned(),
            target: choice.to_p(&players),
        });

        let target = match (role, choice) {
            (_, Choice::Abstain) => Target::Abstain,
            (Role::COP, Choice::Player(p)) => Target::Investigate(p),
            (Role::DOCTOR, Choice::Player(p)) => Target::Save(p),
            (Role::STRIPPER, Choice::Player(p)) => Target::Strip(p),
            _ => panic!("Shouldn't be able to target with this role"),
        };
        self.targets.insert(actor, target);

        self.resolve_dawn(players, comm)
    }

    pub fn resolve_mark<U: RawPID>(
        &mut self,
        players: &Vec<Player<U>>,
        killer: Pidx,
        mark: Choice<Pidx>,
        comm: &Comm<U>,
    ) -> Option<NightResolution<U>> {
        // If killer has already targeted tonight, retract that target.
        if let Entry::Occupied(mut e) = self.targets.entry(killer) {
            *e.get_mut() = Target::Abstain;
        }

        self.scheme = match mark {
            Choice::Player(p) => Some(Mark::Kill(killer, p)),
            Choice::Abstain => Some(Mark::Abstain),
        };

        comm.tx(Event::Mark {
            killer: players[killer].to_owned(),
            mark: mark.to_p(players),
        });
        self.resolve_dawn(players, comm)
    }

    pub fn resolve_dawn<U: RawPID>(
        &mut self,
        players: &Vec<Player<U>>,
        comm: &Comm<U>,
    ) -> Option<NightResolution<U>> {
        type T = Targets;

        let night_action_players = get_players_that(players, |(_, p)| p.role.targeting()).count();
        let night_actions = self.targets.len();
        if night_actions < night_action_players || self.scheme.is_none() {
            return None;
        }

        comm.tx(Event::Dawn);

        let targets = self.targets.to_owned();

        // Take strips
        let (strips, mut targets): (T, T) = targets
            .into_iter()
            .partition(|(_, t)| matches!(t, Target::Strip(_)));

        // Collect Strips
        let mut block_map = HashMap::new();
        for (stripper, target) in strips {
            if let Target::Strip(stripped) = target {
                // RULE StripNotify Always
                block_map
                    .entry(stripped)
                    .or_insert_with(Vec::new)
                    .push(stripper);
            }
        }
        for (actor, target) in &mut targets {
            if let Entry::Occupied(e) = block_map.entry(*actor) {
                match target {
                    Target::Save(_) | Target::Investigate(_) => {
                        // RULE StripNotify Useful
                        strip_events(&comm, e.get(), *actor, &players);
                        *target = Target::Abstain;
                    }
                    _ => {}
                }
            }
        }

        // Take saves
        let (saves, targets): (T, T) = targets
            .into_iter()
            .partition(|(_, t)| matches!(t, Target::Save(_)));

        // Collect saves
        let mut save_map = HashMap::new();
        for (doctor, target) in saves {
            if let Target::Save(saved) = target {
                // RULE SaveSelf
                save_map.entry(saved).or_insert_with(Vec::new).push(doctor);
            }
        }

        // Take Investigations
        let (searches, _): (T, T) = targets
            .into_iter()
            .partition(|(_, t)| matches!(t, Target::Investigate(_)));

        // Enact Investigations
        for (cop, target) in searches {
            if let Target::Investigate(suspect) = target {
                let (cop, suspect, role) = (
                    players[cop].to_owned(),
                    players[suspect].to_owned(),
                    players[suspect].role.to_owned(),
                );
                comm.tx(Event::Investigate { cop, suspect, role })
            }
        }

        let next_phase = Phase::new_day(
            self.night_no + 1,
            block_map.keys().into_iter().copied().collect(),
        );

        // Enact Kill
        let night_resolution = match self.scheme {
            Some(Mark::Kill(killer, mark)) => {
                if let Entry::Occupied(e) = save_map.entry(mark) {
                    save_events(comm, e.get(), killer, mark, players);

                    NightResolution::NoKill(next_phase)
                } else {
                    NightResolution::Kill(killer, mark, next_phase)
                }
            }
            _ => NightResolution::NoKill(next_phase),
        };
        match night_resolution {
            NightResolution::NoKill(_) => {
                comm.tx(Event::NoKill);
            }
            NightResolution::Kill(killer, mark, _) => {
                let (killer, mark) = (players[killer].to_owned(), players[mark].to_owned());
                comm.tx(Event::Kill { killer, mark });
            }
        }
        Some(night_resolution)
    }
}

fn strip_events<U: RawPID>(
    comm: &Comm<U>,
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

fn save_events<U: RawPID>(
    comm: &Comm<U>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum Phase<U: RawPID> {
    Init,
    Day(Day),
    Night(Night),
    End(Team, Vec<ContractResult<U>>),
}

impl<U: RawPID> Phase<U> {
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
            Phase::End(..) => PhaseKind::End,
        }
    }
    pub fn is_day(&mut self) -> Result<&mut Day, InvalidActionError<U>> {
        if let Phase::Day(day) = self {
            Ok(day)
        } else {
            Err(InvalidActionError::InvalidPhase {
                expected: PhaseKind::Day,
                found: self.to_owned(),
            })
        }
    }
    pub fn is_night(&mut self) -> Result<&mut Night, InvalidActionError<U>> {
        if let Phase::Night(night) = self {
            Ok(night)
        } else {
            Err(InvalidActionError::InvalidPhase {
                expected: PhaseKind::Night,
                found: self.to_owned(),
            })
        }
    }

    pub fn next_phase(&mut self, next_phase: Phase<U>, players: &Vec<Player<U>>, comm: &Comm<U>) {
        *self = next_phase;

        match self {
            Phase::Day(Day { day_no, .. }) => comm.tx(Event::Day {
                day_no: *day_no,
                players: players.clone(),
            }),
            Phase::Night(Night { night_no, .. }) => comm.tx(Event::Night {
                night_no: *night_no,
                players: players.clone(),
            }),
            Phase::End(winner, contract_results) => comm.tx(Event::End {
                winner: *winner,
                contract_results: contract_results.to_owned(),
            }),
            _ => panic!("Should never go to Init Phase!"),
        }
    }
}
impl<U: RawPID> Display for Phase<U> {
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
            Phase::End(winner, contracts) => {
                write!(f, "End: {:?}, contracts: {:?}", winner, contracts)
            }
        }
    }
}

// mod test {
//     use super::*;

//     fn _basics() -> Comm<u64, String> {
//         let (_, rx) = std::sync::mpsc::channel();
//         let (tx, _) = std::sync::mpsc::channel();

//         let comm: Comm<u64, String> = Comm::new(rx, tx);
//         return comm;
//     }

//     #[test]
//     fn phase_next_phase() {
//         let mut phase = Phase::new_night(1);
//         let comm = _basics();

//         phase.next_phase(Phase::new_day(2, vec![]), &comm);

//         assert!(matches!(phase, Phase::Day(Day{day_no, .. }) if day_no == 2));
//     }
// }
