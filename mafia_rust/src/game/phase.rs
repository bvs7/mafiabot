use serde::Serialize;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display};

use super::comm::*;
use super::player::*;
use super::*;

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

pub enum DayResolution {
    Elected(Pidx, Vec<Pidx>, Pidx, Phase),
    NoKill(Phase),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Day {
    pub day_no: usize,
    pub votes: Votes,
    pub blocked: Vec<Pidx>,
}

impl Day {
    pub fn resolve_vote<U: RawPID, S: Source>(
        &mut self,
        players: &Players<U>,
        (voter, ballot): (Pidx, Option<Choice<Pidx>>),
        comm: &Comm<U, S>,
    ) -> Option<DayResolution> {
        let former = self
            .votes
            .iter()
            .position(|(v, _)| v == &voter)
            .map(|i| self.votes.remove(i))
            .map(|(_, b)| b);

        let ballot = match ballot {
            Some(choice) => {
                self.votes.push((voter, choice));
                choice
            }
            None => {
                comm.tx(Event::Retract {
                    voter: players[voter],
                    former: former.map(|c| c.to_p(players)),
                });
                return None; // Vote retraction can't cause election
            }
        };

        let n_players = players.len();
        let threshold = match ballot {
            Choice::Player(_) => n_players / 2 + 1,
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

        let &hammer = electors.last().expect("At least one elector");

        let electors_p: Vec<Player<U>> = electors.iter().map(|e| players[*e].to_owned()).collect();

        comm.tx(Event::Election {
            electors: electors_p,
            ballot: ballot.to_p(&players),
        });

        let next_phase = Phase::new_night(self.day_no);
        if let Choice::Player(elected) = ballot {
            Some(DayResolution::Elected(
                elected, electors, hammer, next_phase,
            ))
        } else {
            Some(DayResolution::NoKill(next_phase))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Target {
    Strip(Pidx),
    Save(Pidx),
    Investigate(Pidx),
    Abstain,
}

type Targets = HashMap<Pidx, Target>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Night {
    night_no: usize,
    pub targets: Targets,
    pub scheme: Option<(Pidx, Choice<Pidx>)>,
}

pub enum NightResolution {
    NoKill(Phase),
    Kill(Pidx, Pidx, Phase),
}

impl Night {
    pub fn resolve_target<U: RawPID, S: Source>(
        &mut self,
        players: &Players<U>,
        actor: Pidx,
        target: Choice<Pidx>,
        role: Role,
        comm: &Comm<U, S>,
    ) -> Option<NightResolution> {
        // If actor has already targeted tonight, retract that target.
        if let Some((killer, _)) = self.scheme {
            if killer == actor {
                self.scheme = Some((killer, Choice::Abstain));
            }
        }
        comm.tx(Event::Target {
            actor: players[actor].to_owned(),
            target: target.to_p(&players),
        });

        let t = match (role, target) {
            (_, Choice::Abstain) => Target::Abstain,
            (Role::COP, Choice::Player(p)) => Target::Investigate(p),
            (Role::DOCTOR, Choice::Player(p)) => Target::Save(p),
            (Role::STRIPPER, Choice::Player(p)) => Target::Strip(p),
            _ => panic!("Shouldn't be able to target with this role"),
        };
        self.targets.insert(actor, t);

        self.resolve_dawn(players, comm)
    }

    pub fn resolve_mark<U: RawPID, S: Source>(
        &mut self,
        players: &Players<U>,
        killer: Pidx,
        mark: Choice<Pidx>,
        comm: &Comm<U, S>,
    ) -> Option<NightResolution> {
        // If killer has already targeted tonight, retract that target.
        if let Entry::Occupied(mut e) = self.targets.entry(killer) {
            *e.get_mut() = Target::Abstain;
        }

        self.scheme = Some((killer, mark));

        comm.tx(Event::Mark {
            killer: players[killer].to_owned(),
            mark: mark.to_p(players),
        });
        self.resolve_dawn(players, comm)
    }

    pub fn resolve_dawn<U: RawPID, S: Source>(
        &mut self,
        players: &Players<U>,
        comm: &Comm<U, S>,
    ) -> Option<NightResolution> {
        type T = Targets;

        let night_action_players = get_players_that(players, |(_, p)| p.role.targeting()).count();
        let night_actions = self.targets.len();
        if (night_actions < night_action_players || self.scheme.is_none()) {
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
                        Game::strip_events(&comm, e.get(), *actor, &players);
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
                let (cop, suspect, role) = (players[cop], players[suspect], players[suspect].role);
                comm.tx(Event::Investigate { cop, suspect, role })
            }
        }

        let next_phase = Phase::new_day(
            self.night_no + 1,
            block_map.keys().into_iter().copied().collect(),
        );

        // Enact Kill
        let night_resolution = match self.scheme {
            Some((killer, Choice::Player(mark))) => {
                if let Entry::Occupied(e) = save_map.entry(mark) {
                    Game::save_events(comm, e.get(), killer, mark, players);

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
                let (killer, mark) = (players[killer], players[mark]);
                comm.tx(Event::Kill { killer, mark });
            }
        }
        Some(night_resolution)
    }
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

    pub fn eliminate<U: RawPID, S: Source>(
        &mut self,
        players: &mut Players<U>,
        to_die: &[Pidx],
        _proxy: Pidx,
        comm: &Comm<U, S>,
    ) -> Option<Phase> {
        let mut to_die = to_die.to_owned();
        to_die.sort();
        // Remove from largest to smallest to avoid invalidating indices
        for p in to_die.into_iter().rev() {
            let player = players[p].to_owned();
            comm.tx(Event::Eliminate { player });

            players.remove(p);
        }
        // all Pidxs are now invalid...
        self.clear();

        let winner = check_team_numbers(players);

        winner.map(|w| Phase::End(w))
    }

    pub fn next_phase<U: RawPID, S: Source>(&mut self, next_phase: Phase, comm: &Comm<U, S>) {
        *self = next_phase;

        match *self {
            Phase::Day(Day { day_no, .. }) => comm.tx(Event::Day { day_no }),
            Phase::Night(Night { night_no, .. }) => comm.tx(Event::Night { night_no }),
            Phase::End(winner) => comm.tx(Event::End { winner }),
            _ => panic!("Should never go to Init Phase!"),
        }

        if let SaveStrategy::PerPhase(fname) = &comm.save {
            // self.save_game(fname).expect("Saving game should work");
        };
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
