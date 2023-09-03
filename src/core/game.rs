use std::collections::{HashMap, HashSet};

use crate::prelude::*;

use super::Role;
use super::*;

impl Players {
    pub fn check(&self, player: PID) -> Result<()> {
        if !self.0.contains_key(&player) {
            return Err(Error::Generic(format!("Player {} not found", player)));
        }
        Ok(())
    }

    pub fn role(&self, player: PID) -> Result<Role> {
        self.check(player)?;
        Ok(self.0[&player])
    }

    pub fn threshold(&self, choice: Choice) -> usize {
        match choice {
            Choice::Player(_) => self.0.len() / 2 + 1,
            Choice::Abstain => (self.0.len() + 1) / 2,
        }
    }
}

impl IntoIterator for Players {
    type Item = (PID, Role);
    type IntoIter = std::collections::hash_map::IntoIter<PID, Role>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/* #region Game Data Types */

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Kinded)]
pub enum Phase {
    Day {
        day: usize,
        votes: HashMap<PID, Choice>,
        blocks: HashSet<PID>,
    },
    Night {
        day: usize,
        targets: HashMap<PID, Choice>,
        scheme: Option<(PID, Choice)>,
    },
    Dusk {
        day: usize,
        avenger: PID,
        hammer: PID,
        voters: HashSet<PID>,
    },
    End {
        winner: Team,
    },
}

impl Phase {
    pub fn new_day(day: usize, blocks: HashSet<PID>) -> Self {
        Phase::Day {
            day,
            votes: HashMap::new(),
            blocks,
        }
    }
    pub fn new_night(day: usize) -> Self {
        Phase::Night {
            day,
            targets: HashMap::new(),
            scheme: None,
        }
    }
    pub fn new_dusk(day: usize, avenger: PID, hammer: PID, voters: HashSet<PID>) -> Self {
        Phase::Dusk {
            day,
            avenger,
            hammer,
            voters,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Players(pub HashMap<PID, Role>);

#[derive(Debug, Clone)]
pub struct Game {
    id: u64,
    players: Players,
    phase: Phase,
    role_history: HashMap<PID, Vec<Role>>,
    rules: Rules,
    eo: EventOutput,
}

impl From<Election> for Choice {
    fn from(election: Election) -> Self {
        match election {
            Election::Hammer(votee, _) => Choice::Player(votee),
            Election::Peace => Choice::Abstain,
        }
    }
}

impl From<(PID, Choice)> for Election {
    fn from((voter, choice): (PID, Choice)) -> Self {
        match choice {
            Choice::Player(votee) => Election::Hammer(votee, voter),
            Choice::Abstain => Election::Peace,
        }
    }
}

pub enum ActionResult {
    TryElection(Election),
    TryDawn,
}

/* #endregion Game Data Types */

/* #region Game Impl */

impl Game {
    pub fn handle_action(&mut self, action: Action) -> Result<Option<ActionResult>> {
        Ok(match action {
            Action::Vote { voter, ballot } => self
                .handle_vote(voter, ballot)?
                .map(|election| ActionResult::TryElection(election)),
            Action::Reveal { actor } => {
                self.handle_reveal(actor)?;
                None
            }
            Action::Target { actor, target } => self
                .handle_target(actor, target)?
                .then_some(ActionResult::TryDawn),
            Action::Mark { actor, target } => self
                .handle_mark(actor, target)?
                .then_some(ActionResult::TryDawn),
        })
    }

    /// Returns a Choice if an election threshold was reached
    pub fn handle_vote(&mut self, voter: PID, ballot: Option<Choice>) -> Result<Option<Election>> {
        // Validate voter
        self.players.check(voter)?;
        // Check for dusk vote
        if self.dusk_vote(voter, ballot)? {
            return Ok(None);
        }
        // Validate phase
        let Phase::Day { votes, .. } = &mut self.phase else {
            return Err(Error::Generic("Invalid phase for vote".to_string()));
        };
        // Handle vote
        let result = match ballot {
            Some(choice) => {
                match choice {
                    Choice::Player(votee) => {
                        self.players.check(votee)?;
                    }
                    Choice::Abstain => {}
                }
                votes.insert(voter, choice);

                self.check_election(Some((voter, choice).into()), false)
                    .map(|(e, _)| e)
            }
            None => {
                votes.remove(&voter);
                None
            }
        };
        self.eo.send(Event::Vote { voter, ballot })?;
        return Ok(result);
    }

    pub fn handle_reveal(&self, actor: PID) -> Result<()> {
        self.players.check(actor)?;
        let Phase::Day { blocks, .. } = &self.phase else {
            return Err(Error::Generic("Invalid phase for reveal".to_string()));
        };
        if blocks.contains(&actor) {
            self.eo.send(Event::Block { blocked: actor });
            return Ok(());
        }
        let role = self.players.role(actor)?;
        self.eo.send(Event::Reveal {
            player: actor,
            role,
        })?;
        Ok(())
    }

    pub fn handle_target(&mut self, actor: PID, target: Choice) -> Result<bool> {
        self.players.check(actor)?;
        let Phase::Night {
            targets,
            scheme: mark,
            ..
        } = &mut self.phase
        else {
            return Err(Error::Generic("Invalid phase for target".to_string()));
        };
        let role = self.players.role(actor)?;
        if !role.targeting() {
            return Err(Error::Generic("Invalid role for targeting".to_string()));
        }
        targets.insert(actor, target);
        self.eo.send(Event::Target { actor, target })?;

        Ok(self.check_dawn())
    }

    pub fn handle_mark(&mut self, actor: PID, target: Choice) -> Result<bool> {
        self.players.check(actor)?;
        let Phase::Night {
            targets,
            scheme: mark,
            ..
        } = &mut self.phase
        else {
            return Err(Error::Generic("Invalid phase for mark".to_string()));
        };
        let role = self.players.role(actor)?;
        if role.team() != Team::Mafia {
            return Err(Error::Generic("Invalid role for marking".to_string()));
        }
        if role == Role::GOON && target != Choice::Player(actor) {
            return Err(Error::Generic(
                "Invalid target for GOON marking".to_string(),
            ));
        }
        *mark = Some((actor, target));
        Ok(self.check_dawn())
    }

    pub fn dusk_vote(&mut self, voter: PID, ballot: Option<Choice>) -> Result<bool> {
        let Phase::Dusk {
            day,
            avenger,
            hammer,
            voters,
        } = self.phase.clone()
        else {
            return Ok(false);
        };
        if avenger != voter {
            return Err(Error::Generic("Only the avenger can vote".to_string()));
        }
        match ballot {
            Some(Choice::Player(votee)) if voters.contains(&votee) => {
                self.players.check(votee)?;
                self.eo.send(Event::Revenge {
                    avenger: voter,
                    votee,
                })?;
                // Eliminate votee then avenger
                self.eliminate(votee, voter);
                self.eliminate(avenger, hammer);
                // Phase to Night
                self.phase = Phase::new_night(day);
                Ok(true)
            }
            _ => Err(Error::Generic(
                "Must vote for a player who voted for you".to_string(),
            )),
        }
    }

    pub fn eliminate(&mut self, player: PID, proxy: PID) {
        let role = self.players.role(player).unwrap();
        self.players.0.remove(&player);
        self.eo.send(Event::Reveal { player, role });

        // Check for refocusing roles!
        for (player, former_role) in self.players.clone() {
            match role {
                Role::GUARD(charge) | Role::AGENT(charge) if charge == player => {
                    // Refocus to AGENT or IDIOT...
                    let new_role = former_role.refocus(player, proxy);
                    self.players.0.insert(player, new_role);
                    // push new role to history
                    let entry = self.role_history.entry(player).or_default();
                    entry.push(new_role);
                    self.eo.send(Event::Refocus {
                        holder: player,
                        former_role,
                        new_role,
                    });
                }
                _ => {}
            }
        }
    }

    pub fn kill(&mut self, mark: PID, killer: PID) {
        self.eo.send(Event::Kill { killer, mark });
        self.eliminate(mark, killer);
    }

    /// Check for an election
    /// @choice: If present, the choice to check. If not present
    ///     check all possible choices
    /// @return: If an election was found, the winning choice
    pub fn check_election(&self, e: Option<Election>, f: bool) -> Option<(Election, HashSet<PID>)> {
        let Phase::Day { day, votes, .. } = &self.phase else {
            return None;
        };
        // collect voters
        let Some(election) = e else { todo!() };
        let choice: Choice = election.into();
        let voters: HashSet<_> = votes
            .iter()
            .filter_map(|(v, c)| (c == &choice).then_some(*v))
            .collect();
        let threshold = self.players.threshold(choice);
        (f || voters.len() >= threshold).then(|| (election, voters))
    }

    pub fn elect(&mut self, election: Election, voters: HashSet<PID>) {
        // Collect voters
        let Phase::Day { day, votes, .. } = &self.phase else {
            unreachable!("Invalid phase for election");
        };
        // split from phase borrow.
        let day = day.clone();

        self.eo.send(Event::Election {
            election,
            voters: voters.clone(),
        });

        match election {
            Election::Hammer(elected, hammer) => {
                let role = self.players.role(elected).unwrap();
                if role.kind() == RoleKind::IDIOT {
                    let new_role = Role::IDIOT(true);
                    self.players.0.insert(elected, new_role);
                    // push new role to history
                    let entry = self.role_history.entry(elected).or_default();
                    entry.push(new_role);

                    self.eo.send(Event::Reveal {
                        player: elected,
                        role,
                    });
                    self.phase = Phase::new_dusk(day, elected, hammer, voters.clone());
                    self.eo.send(Event::Dusk {
                        day,
                        avenger: elected,
                        voters: voters.clone(),
                    });
                    return;
                } else {
                    // normal election
                    self.eliminate(elected, hammer);
                }
            }
            Election::Peace => {}
        }
        self.phase = Phase::new_night(day);
    }

    pub fn check_dawn(&self) -> bool {
        let Phase::Night {
            targets,
            scheme: mark,
            ..
        } = &self.phase
        else {
            unreachable!("Invalid phase for dawn check");
        };
        for (player, role) in self.players.clone().into_iter() {
            if role.targeting() && !targets.contains_key(&player) {
                return false;
            }
        }

        return mark.is_some();
    }

    pub fn dawn(&mut self) {
        // Collect targets
        let Phase::Night {
            targets,
            scheme,
            day,
        } = self.phase.clone()
        else {
            unreachable!("Invalid phase for dawn");
        };

        let day = day.clone();

        /// Collect targets
        let mut stripped = HashMap::<PID, HashSet<PID>>::new();
        let mut saveds = HashMap::<PID, HashSet<PID>>::new();
        let mut searches = HashSet::<(PID, PID)>::new();
        for (actor, choice) in targets {
            let role = self.players.role(actor).expect("Targeter should be found");
            match (role.kind(), choice) {
                (RoleKind::STRIPPER, Choice::Player(target)) => {
                    let entry = stripped.entry(target).or_insert_with(HashSet::new);
                    entry.insert(actor);
                }
                (RoleKind::DOCTOR, Choice::Player(target)) => {
                    let entry = saveds.entry(target).or_insert_with(HashSet::new);
                    entry.insert(actor);
                }
                (RoleKind::COP, Choice::Player(target)) => {
                    searches.insert((actor, target));
                }
                _ => {}
            }
        }

        // Create Blocks
        let mut blocks: HashSet<PID> = stripped.keys().cloned().collect();
        let mut killed = None;

        // Check schemes and saves
        if let Some((killer, Choice::Player(mark))) = scheme {
            // Check if mark was saved
            let mut saved = false;
            if let Some(savers) = saveds.get(&mark) {
                for &doctor in savers {
                    match stripped.get(&doctor) {
                        Some(doc_strippers) => {
                            // Successful strip
                            for &stripper in doc_strippers {
                                self.eo.send(Event::Strip {
                                    stripper: stripper,
                                    target: doctor.clone(),
                                });
                            }
                            self.eo.send(Event::Block {
                                blocked: doctor.clone(),
                            });
                        }
                        None => {
                            // Successful save
                            saved = true;
                            self.eo.send(Event::Save {
                                doctor,
                                target: mark.clone(),
                            });
                        }
                    }
                }
            }
            if !saved {
                killed = Some((killer, mark));
            }
        }

        // Perform investigations
        for (cop, target) in searches {
            if let Some((_, mark)) = killed {
                if mark == cop || mark == target {
                    continue; // Someone killed before investigation could occur
                }
            }

            if let Some(cop_strippers) = stripped.get(&cop) {
                // Successful strip
                for &stripper in cop_strippers {
                    self.eo.send(Event::Strip {
                        stripper,
                        target: cop.clone(),
                    });
                }
                self.eo.send(Event::Block {
                    blocked: cop.clone(),
                });
                continue;
            }

            let role = self.players.role(target).expect("Target should be found");
            self.eo.send(Event::Investigate { cop, target, role });
        }

        // Eliminate killed
        if let Some((killer, mark)) = killed {
            self.kill(mark, killer);
        }

        // Phase to Day
        self.phase = Phase::new_day(day + 1, blocks);
        self.eo.send(Event::Day {
            day: day + 1,
            players: self.players.clone().into_iter().collect(),
        });
    }
}

/* #endregion Game Impl */

pub struct GameWrapper(Arc<Mutex<Game>>);

impl GameWrapper {
    pub fn handle_action(&self, action: Action) -> Result<()> {
        let mut game = self.0.lock().unwrap();

        match game.handle_action(action)? {
            Some(ActionResult::TryElection(election)) => {
                // Schedule an election check
                let game_lock = self.0.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    try_election(game_lock, Some(election), false);
                });
            }
            Some(ActionResult::TryDawn) => {
                // Schedule a dawn check
                let game_lock = self.0.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    try_dawn(game_lock, false);
                });
            }
            _ => {}
        }
        Ok(())
    }
}

/// Spawned in a thread, this will check for and possibly force an election
/// @game: The game monitor
/// @choice: The choice to check votes for. If None, will check all choices
/// @force: If true, will force an election of choice, even if no election
fn try_election(game: Arc<Mutex<Game>>, election: Option<Election>, force: bool) {
    let mut game = game.lock().unwrap();
    if let Some((election, voters)) = game.check_election(election, force) {
        game.elect(election, voters);
    }
}

fn try_dawn(game: Arc<Mutex<Game>>, force: bool) {
    let mut game = game.lock().unwrap();
    let result = game.check_dawn();
    if force || result {
        game.dawn();
    }
}
