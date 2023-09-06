use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use serde_json::map::IntoIter;

use crate::prelude::*;

use super::Role;
use super::*;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct RoleHist {
    pub role: Role,
    pub history: Vec<Role>,
}

impl RoleHist {
    pub fn set_role(&mut self, new_role: Role) {
        if self.role != new_role {
            self.history.push(self.role);
            self.role = new_role;
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct Players(pub HashMap<PID, RoleHist>);

impl Players {
    pub fn get(&self, player: PID) -> MResult<&RoleHist> {
        if !self.0.contains_key(&player) {
            return Err(Error::Generic(format!("Player {} not found", player)));
        }
        Ok(&self.0[&player])
    }

    pub fn role(&self, player: PID) -> MResult<Role> {
        let rh = self.get(player)?;
        Ok(self.0[&player].role)
    }

    pub fn items(&self) -> std::collections::hash_map::IntoIter<PID, RoleHist> {
        self.0.clone().into_iter()
    }

    pub fn roles(&self) -> std::collections::hash_map::IntoIter<PID, Role> {
        self.0
            .clone()
            .into_iter()
            .map(|(p, rh)| (p, rh.role))
            .collect::<HashMap<_, _>>()
            .into_iter()
    }

    pub fn set_role(&mut self, player: PID, new_role: Role) -> MResult<()> {
        let new_rh = self.get(player)?.clone();
        self.0.insert(player, new_rh);
        return Ok(());
    }

    pub fn threshold(&self, choice: Choice) -> usize {
        match choice {
            Choice::Player(_) => self.0.len() / 2 + 1,
            Choice::Abstain => (self.0.len() + 1) / 2,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SerRole<T> {
    Role(Role_<T>),
    Hist(Vec<Role_<T>>),
}

impl Serialize for SerRole<String> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SerRole::Role(role) => role.serialize(serializer),
            SerRole::Hist(history) => history.serialize(serializer),
        }
    }
}

impl From<RoleHist> for SerRole<String> {
    fn from(rh: RoleHist) -> Self {
        if rh.history.is_empty() {
            SerRole::Role(rh.role)
        } else {
            let mut all_roles = rh.history.clone();
            all_roles.push(rh.role);
            SerRole::Hist(all_roles)
        }
    }
}

impl From<SerRole<String>> for RoleHist {
    fn from(sr: SerRole<String>) -> Self {
        match sr {
            SerRole::Role(role) => RoleHist {
                role,
                history: Vec::new(),
            },
            SerRole::Hist(history) => {
                let role = history.last().unwrap().clone();
                let history = history[..history.len() - 1].to_vec();
                RoleHist { role, history }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerPlayers(HashMap<String, SerRole<String>>);

type Names = HashMap<PID, String>;

impl From<(Players, &Names)> for SerPlayers {
    fn from((players, names): (Players, &Names)) -> Self {
        let mut map = HashMap::new();
        for (pid, rh) in players.0 {
            let name = names.get(&pid).unwrap().clone();
            map.insert(name, SerRole::from(rh));
        }
        SerPlayers(map)
    }
}

impl From<(SerPlayers, &Names)> for Players {
    fn from((ser_players, names): (SerPlayers, &Names)) -> Self {
        let mut map = HashMap::new();
        for (name, sr) in ser_players.0 {
            let pid = names.iter().find(|(_, n)| *n == &name).unwrap().0;
            map.insert(pid.clone(), RoleHist::from(sr));
        }
        Players(map)
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

#[derive(Debug, Clone)]
pub struct Game {
    id: u64,
    players: Players,
    phase: Phase,
    rules: Rules,
    names: Option<HashMap<PID, String>>,
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
    pub fn handle_action(&mut self, action: Action) -> MResult<Option<ActionResult>> {
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
    pub fn handle_vote(&mut self, voter: PID, ballot: Option<Choice>) -> MResult<Option<Election>> {
        // Validate voter
        self.players.get(voter)?;
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
                        self.players.get(votee)?;
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

    pub fn handle_reveal(&self, actor: PID) -> MResult<()> {
        self.players.get(actor)?;
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

    pub fn handle_target(&mut self, actor: PID, target: Choice) -> MResult<bool> {
        self.players.get(actor)?;
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

    pub fn handle_mark(&mut self, killer: PID, mark: Choice) -> MResult<bool> {
        self.players.get(killer)?;
        let Phase::Night {
            targets, scheme, ..
        } = &mut self.phase
        else {
            return Err(Error::Generic("Invalid phase for mark".to_string()));
        };
        let role = self.players.role(killer)?;
        if role.team() != Team::Mafia {
            return Err(Error::Generic("Invalid role for marking".to_string()));
        }
        if role == Role::GOON && mark != Choice::Player(killer) {
            return Err(Error::Generic(
                "Invalid target for GOON marking".to_string(),
            ));
        }
        *scheme = Some((killer, mark));
        self.eo.send(Event::Mark { killer, mark })?;
        Ok(self.check_dawn())
    }

    pub fn dusk_vote(&mut self, voter: PID, ballot: Option<Choice>) -> MResult<bool> {
        let Phase::Dusk {
            day,
            avenger,
            hammer,
            ref voters,
        } = self.phase
        else {
            return Ok(false);
        };
        if avenger != voter {
            return Err(Error::Generic("Only the avenger can vote".to_string()));
        }
        match ballot {
            Some(Choice::Player(votee)) if voters.contains(&votee) => {
                self.players.get(votee)?;
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
                    self.players.set_role(elected, new_role);

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

        let (stripped, saveds, searches) = self.collect_targets(&targets);

        let killed = self.check_scheme(&scheme, &stripped, &saveds);

        self.perform_investigations(&stripped, &searches, &killed);

        // Eliminate killed
        if let Some((killer, mark)) = killed {
            self.kill(mark, killer);
        }

        // Phase to Day
        self.phase = Phase::new_day(day + 1, stripped.keys().cloned().collect());
        self.eo.send(Event::Day {
            day: day + 1,
            players: self.players.roles().collect(),
        });
    }

    pub fn kill(&mut self, mark: PID, killer: PID) {
        self.eo.send(Event::Kill { killer, mark });
        self.eliminate(mark, killer);
    }

    pub fn eliminate(&mut self, player: PID, proxy: PID) {
        let role = self.players.role(player).unwrap();
        self.players.0.remove(&player);
        self.eo.send(Event::Reveal { player, role });

        // Check for refocusing roles!
        for (player, former_role) in self.players.roles() {
            match role {
                Role::GUARD(charge) | Role::AGENT(charge) if charge == player => {
                    // Refocus to AGENT or IDIOT...
                    let new_role = former_role.refocus(player, proxy);
                    self.players.set_role(player, new_role);
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

    /// Check for an election
    /// @choice: If present, the choice to check. If not present
    ///     check all possible choices
    /// @return: If an election was found, the winning choice
    pub fn check_election(&self, e: Option<Election>, f: bool) -> Option<(Election, HashSet<PID>)> {
        let Phase::Day { day, votes, .. } = &self.phase else {
            return None;
        };
        // collect voters
        let Some(election) = e else {
            // Refactor votes to be ordered?
            // Then we can find the newest vote for each Player, and if Abstain is present
            todo!()
        };
        let choice: Choice = election.into();
        let voters: HashSet<_> = votes
            .iter()
            .filter_map(|(v, c)| (c == &choice).then_some(*v))
            .collect();
        let threshold = self.players.threshold(choice);
        (f || voters.len() >= threshold).then(|| (election, voters))
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
        for (player, role) in self.players.roles() {
            if role.targeting() && !targets.contains_key(&player) {
                return false;
            }
        }

        return mark.is_some();
    }

    fn collect_targets(
        &self,
        targets: &HashMap<PID, Choice>,
    ) -> (
        HashMap<PID, HashSet<PID>>,
        HashMap<PID, HashSet<PID>>,
        HashSet<(PID, PID)>,
    ) {
        /// Collect targets
        let mut stripped = HashMap::<PID, HashSet<PID>>::new();
        let mut saveds = HashMap::<PID, HashSet<PID>>::new();
        let mut searches = HashSet::<(PID, PID)>::new();
        for (&actor, &choice) in targets {
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
        return (stripped, saveds, searches);
    }

    fn check_scheme(
        &self,
        scheme: &Option<(PID, Choice)>,
        stripped: &HashMap<PID, HashSet<PID>>,
        saveds: &HashMap<PID, HashSet<PID>>,
    ) -> Option<(PID, PID)> {
        // Check schemes and saves
        if let &Some((killer, Choice::Player(mark))) = scheme {
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
                return Some((killer, mark));
            }
        }
        None
    }
    fn perform_investigations(
        &self,
        stripped: &HashMap<PID, HashSet<PID>>,
        searches: &HashSet<(PID, PID)>,
        killed: &Option<(PID, PID)>,
    ) {
        for &(cop, target) in searches {
            if let &Some((_, mark)) = killed {
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
    }
}

impl Serialize for Game {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        todo!()
    }
}

/* #endregion Game Impl */

pub struct GameWrapper(Arc<Mutex<Game>>);

impl GameWrapper {
    pub fn handle_action(&self, action: Action) -> MResult<()> {
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
