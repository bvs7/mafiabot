// don't warn about unused imports
#![allow(dead_code, unused_variables, unused_imports)]

pub mod base;
pub mod interface;
pub mod roles;
pub mod timer;

use base::{Choice, ID};
use interface::{Action, CoreError, Event, EventOutput};
use roles::{DawnState, DawnStateChange, Role, RoleKind, Team};
use timer::Timer;

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NightAction<PID: ID> {
    actor: PID,
    role: Role<PID>,
    target: PID,
    priority: i8,
}

impl<PID: ID> PartialOrd for NightAction<PID> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl<PID: ID> Ord for NightAction<PID> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

// TODO: move this to a new file
#[derive(Debug, Clone, Copy)]
pub struct Rules {}

// Maintains historical data about the game
// Used for revealing information about the game
#[derive(Debug, Clone)]
pub struct Stats<PID: ID> {
    role_history: HashMap<PID, Vec<Role<PID>>>,
}

impl<PID: ID> Stats<PID> {
    fn new() -> Self {
        Stats {
            role_history: HashMap::new(),
        }
    }
}

#[derive(EnumKind, Debug)]
#[enum_kind(PhaseKind)]
pub enum Phase<PID: ID> {
    Init,
    Day {
        votes: HashMap<PID, Choice<PID>>, // voter -> choice
        blocks: HashMap<PID, Vec<PID>>,   // blocked -> blockers
        elect_timer: Option<Timer<Choice<PID>>>,
    },
    Night {
        targets: HashMap<PID, Choice<PID>>, // actor -> target
        scheme: Option<(PID, Choice<PID>)>, // actor -> (target, choice)
        dawn_timer: Option<Timer<()>>,
    },
    Eclipse {
        avenger: PID,
        hammer: PID,
        options: Vec<PID>,
    },
    End {
        winner: Team,
    },
}

impl<PID: ID> Phase<PID> {
    fn kind(&self) -> PhaseKind {
        return PhaseKind::from(self);
    }
}

#[derive(Debug)]
struct State<PID: ID> {
    day_no: u32,
    players: HashMap<PID, Role<PID>>,
    phase: Phase<PID>,
    role_history: HashMap<PID, Vec<Role<PID>>>,
}

impl<PID: ID> State<PID> {
    pub fn new(players: HashMap<PID, Role<PID>>) -> Self {
        let day_no = 0;
        let phase = Phase::Init;
        let role_history = HashMap::new();
        State {
            day_no,
            players,
            phase,
            role_history,
        }
    }
}

#[derive(Debug)]
pub struct Core<PID: ID, GID: ID> {
    id: GID,
    state: State<PID>,
    rules: Rules,
    event_tx: EventOutput<PID>,
    action_rx: Receiver<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
    action_tx: Sender<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
}

/*
Core game loop:
- Take in an action + response channel
- process the action and return the response

Additionaly, the action could spawn some kind of timer. That timer might itself throw an action
*/

// Idea: Use tokio::sync::oneshot for the Action responses

// We will have Actions for grabbing status? Or maybe we have two commands. Either an Action or just a Status command.
//  This will probably just copy state and send it back in the oneshot response.
impl<'a, PID: ID, GID: ID> Core<PID, GID> {
    pub fn new(
        id: GID,
        players: HashMap<PID, Role<PID>>,
        rules: Rules,
        event_tx: EventOutput<PID>,
        action_rx: Receiver<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
        action_tx: Sender<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
    ) -> Self {
        let day_no = 0;

        let state = State::new(players);

        let mut core = Core {
            id,
            state,
            rules,
            event_tx,
            action_rx,
            action_tx,
        };

        core.start().expect("No send error during init");
        return core;
    }

    /// Consumes self
    pub fn start_thread(mut self) {
        thread::spawn(move || {
            self.run();
        });
    }

    pub fn run(&mut self) {
        loop {
            let (action, response) = match self.action_rx.recv() {
                Ok(t) => t,
                Err(RecvError) => {
                    panic!("Action channel error!")
                }
            };

            // Do action
            let resp = match action {
                Action::Start => self.start(),
                Action::Vote { voter, choice } => self.vote(voter, Some(choice)),
                Action::Unvote { voter } => self.vote(voter, None),
                Action::Reveal { player } => self.reveal(player),
                Action::Target { actor, target } => self.target(actor, target),
                Action::Scheme { actor, mark } => self.scheme(actor, mark),
                Action::Avenge { avenger, victim } => self.avenge(avenger, victim),
                Action::Elect { candidate, hammer } => self.elect(candidate, hammer),
                Action::Dawn => self.dawn(),
                Action::Close => break,
            };

            // Send response
            response.send(resp).unwrap();
        }
        self.event_tx.send(Event::Close).unwrap();
    }

    fn start(&mut self) -> Result<(), CoreError<PID>> {
        let Phase::Init = self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Init;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        // Add initial roles to rolehist
        for (player, role) in &self.state.players {
            self.state.role_history.insert(*player, vec![*role]);
        }
        // For now assume start event
        let n = self.state.players.len();
        if n % 2 == 0 {
            self.to_night()?;
        } else {
            self.to_day(None)?;
        }
        self.event_tx.send(Event::Start {})?;
        Ok(())
    }

    fn vote(&mut self, voter: PID, ballot: Option<Choice<PID>>) -> Result<(), CoreError<PID>> {
        let _ = Self::validate_player(&self.state.players, voter)?;
        if let Some(Choice::Player(player)) = ballot {
            let _ = Self::validate_player(&self.state.players, player)?;
        }
        let n = self.state.players.len();
        // Check if the phase is day
        let Phase::Day {
            votes, elect_timer, ..
        } = &mut self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        // Update votes
        let former_ballot = match ballot {
            Some(choice) => votes.insert(voter, choice),
            None => votes.remove(&voter),
        };

        if former_ballot == ballot {
            return Ok(());
        }

        self.event_tx.send(Event::Vote {
            voter,
            ballot,
            former_ballot,
        })?;

        // Check for a former election and whether it still has quorum
        if let Some(choice) = former_ballot {
            if let None = Self::check_election(votes, n, choice) {
                if let Some(timer) = elect_timer {
                    // Cancel election timer
                    timer.cancel();
                    *elect_timer = None;
                }
            }
        }

        // Check for a new election
        if let Some(candidate) = ballot {
            if let Some(_) = Self::check_election(votes, n, candidate) {
                // Set election timer
                let action_tx = self.action_tx.clone();
                *elect_timer = Some(Timer::new(
                    SystemTime::now() + Duration::from_secs(1),
                    candidate,
                    Duration::from_millis(100),
                    Box::new(move |candidate| {
                        let (tx, rx) = std::sync::mpsc::channel();
                        action_tx
                            .send((
                                Action::Elect {
                                    candidate,
                                    hammer: voter,
                                },
                                tx,
                            ))
                            .unwrap();
                        rx.recv().unwrap().expect("Election failed!");
                    }),
                ));
            }
        }

        Ok(())
    }

    fn reveal(&self, player: PID) -> Result<(), CoreError<PID>> {
        let role = Self::validate_player(&self.state.players, player)?;
        // Check if the role is a celeb
        if role != Role::CELEB {
            let actual = role.kind();
            return Err(CoreError::ExpectedCeleb { actual });
        }
        // Check that Phase is Day
        let Phase::Day { blocks, .. } = &self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        // Check for a reveal block
        if blocks.contains_key(&player) {
            let blocked = player;
            let blockers = blocks[&player].clone();
            self.event_tx
                .send(Event::EvidentBlock { blocked, blockers })?;
            return Ok(());
        }

        self.event_tx.send(Event::Reveal { player, role })?;
        Ok(())
    }

    fn target(&mut self, actor: PID, target: Choice<PID>) -> Result<(), CoreError<PID>> {
        let role = Self::validate_player(&self.state.players, actor)?;
        if !role.is_targeting() {
            let role = role.kind();
            return Err(CoreError::ExpectedTargetingRole { role });
        }
        if let Choice::Player(player) = target {
            let _ = Self::validate_player(&self.state.players, player)?;
        }

        // Check if the phase is night
        let Phase::Night {
            targets, scheme, ..
        } = &mut self.state.phase
        else {
            return Err(CoreError::InvalidPhase {
                actual: self.state.phase.kind(),
                expected: PhaseKind::Night,
            });
        };
        let former_target = targets.insert(actor, target);
        self.event_tx.send(Event::Target { actor, target })?;

        // TODO: if actor was STRIPPER, make sure they can't kill

        self.check_dawn()?;
        Ok(())
    }

    fn scheme(&mut self, actor: PID, mark: Choice<PID>) -> Result<(), CoreError<PID>> {
        let role = Self::validate_player(&self.state.players, actor)?;
        role.validate_scheming()?;
        if let Choice::Player(player) = mark {
            let _ = Self::validate_player(&self.state.players, player)?;
        }

        // Check if the phase is night
        let Phase::Night { scheme, .. } = &mut self.state.phase else {
            return Err(CoreError::InvalidPhase {
                actual: self.state.phase.kind(),
                expected: PhaseKind::Night,
            });
        };
        scheme.replace((actor, mark));
        self.event_tx.send(Event::Scheme { actor, mark })?;

        // TODO: if killer was STRIPPER, make sure they can't target

        self.check_dawn()?;
        Ok(())
    }

    fn elect(&mut self, candidate: Choice<PID>, hammer: PID) -> Result<(), CoreError<PID>> {
        // Ensure the phase is Day
        let n = self.state.players.len();
        let Phase::Day { votes, .. } = &mut self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };
        if let Choice::Player(player) = candidate {
            let role = Self::validate_player(&self.state.players, player)?;
            // Triple check for election?
            let Some(voters) = Self::check_election(votes, n, candidate) else {
                return Err(CoreError::ExpectedElection { candidate });
            };
            self.event_tx.send(Event::Election {
                choice: candidate,
                voters: voters.clone(),
            })?;

            if role.kind() == RoleKind::IDIOT {
                // Go to ECLIPSE
                self.to_eclipse(player, hammer, voters)?;
                return Ok(());
            }

            if self.eliminate(player, hammer)? {
                // Game Over!
                return Ok(());
            }
        }
        self.to_night()?;
        Ok(())
    }

    fn dawn(&mut self) -> Result<(), CoreError<PID>> {
        let Phase::Night {
            targets, scheme, ..
        } = &self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Night;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        self.event_tx.send(Event::Dawn)?;

        // Turn targets into night actions

        let mut early_night_actions: BinaryHeap<NightAction<PID>> = BinaryHeap::new();
        let mut late_night_actions: BinaryHeap<NightAction<PID>> = BinaryHeap::new();

        for (&actor, &target) in targets {
            let role = Self::validate_player(&self.state.players, actor)?;
            let Choice::Player(target) = target else {
                continue;
            };
            let priority = role
                .night_action_priority()
                .expect("Targeting role should have priority");
            let action = NightAction {
                actor,
                role,
                target,
                priority,
            };
            if priority > 0 {
                early_night_actions.push(action);
            } else {
                late_night_actions.push(action);
            }
        }

        let mut dawn_state: DawnState<PID> = DawnState {
            blocks: HashMap::new(),
            saves: HashMap::new(),
            killed: HashMap::new(),
        };

        self.perform_night_actions(early_night_actions, &mut dawn_state)?;

        self.perform_scheme(scheme, &mut dawn_state)?;

        // Perform Kills
        if dawn_state.killed.len() > 0 {
            let mut game_over = false;
            for (&mark, &killer) in &dawn_state.killed {
                game_over = self.eliminate(mark, killer)? | game_over;
            }
            if game_over {
                return Ok(());
            }
        } else {
            self.event_tx.send(Event::NoNightKill)?;
        }

        self.perform_night_actions(late_night_actions, &mut dawn_state)?;

        self.to_day(Some(dawn_state.blocks))?;
        Ok(())
    }

    // Note: Night actions are performed in batches. All actions of a given
    //   priority create their changes at once, then all changes are applied at once.
    fn perform_night_actions(
        &self,
        mut actions: BinaryHeap<NightAction<PID>>,
        dawn_state: &mut DawnState<PID>,
    ) -> Result<(), CoreError<PID>> {
        let mut next = actions.peek();
        while next.is_some() {
            let current_priority = next.map(|f| f.priority).expect("Checked for some above!");
            let mut changes: Vec<DawnStateChange<PID>> = Vec::new();

            while next.is_some_and(|f| f.priority == current_priority) {
                let a = actions.pop().unwrap();
                changes.extend(a.role.night_action(
                    a.actor,
                    a.target,
                    &dawn_state,
                    &self.event_tx,
                )?);
                next = actions.peek();
            }
            dawn_state.apply_changes(changes);
        }
        Ok(())
    }

    fn perform_scheme(
        &self,
        scheme: &Option<(PID, Choice<PID>)>,
        dawn_state: &mut DawnState<PID>,
    ) -> Result<(), CoreError<PID>> {
        // Perform scheme
        if let &Some((killer, Choice::Player(mark))) = scheme {
            // TODO: if killer was blocked or killed, do nothing
            let mut saved = false;
            // Check for saviors
            if let Some(saviors) = dawn_state.saves.get(&mark) {
                for &savior in saviors {
                    // Check if doctor was killed or blocked?
                    if dawn_state.killed.contains_key(&savior) {
                        continue;
                    }
                    if let Some(blockers) = dawn_state.blocks.get(&savior) {
                        self.event_tx.send(Event::EvidentBlock {
                            blocked: savior,
                            blockers: blockers.clone(),
                        })?;
                        continue;
                    }
                    saved = true;
                    self.event_tx.send(Event::EvidentSave { savior, mark })?;
                }
            }
            if !saved {
                dawn_state.killed.insert(mark, killer);
                self.event_tx.send(Event::Kill { killer, mark })?;
            }
        }
        Ok(())
    }

    fn avenge(&mut self, avenger: PID, victim: Choice<PID>) -> Result<(), CoreError<PID>> {
        let _ = Self::validate_player(&self.state.players, avenger)?;
        let Phase::Eclipse {
            avenger: expected,
            hammer,
            ref options,
        } = self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Eclipse;
            return Err(CoreError::InvalidPhase { actual, expected });
        };
        if avenger != expected {
            return Err(CoreError::ExpectedPlayer {
                actual: avenger,
                expected,
            });
        }

        // default target is hammer
        let mut target = hammer;

        if let Choice::Player(player) = victim {
            let _ = Self::validate_player(&self.state.players, player)?;
            if !options.contains(&player) {
                return Err(CoreError::InvalidOption {
                    actual: player,
                    options: options.clone(),
                });
            }
            target = player;
        }

        self.event_tx.send(Event::Avenge { avenger, target })?;

        self.eliminate(target, avenger)?;

        // change IDIOT's role to win state
        self.refocus(avenger, Role::IDIOT(true))?;

        self.eliminate(avenger, hammer)?;

        self.to_night()?;
        Ok(())
    }

    fn eliminate(&mut self, player: PID, proxy: PID) -> Result<bool, CoreError<PID>> {
        let role = Self::validate_player(&self.state.players, player)?;

        // Check contracting roles
        let mut updates: Vec<(PID, Role<PID>)> = Vec::new();
        for (&contractor, &role) in &self.state.players {
            let new_role = match role {
                Role::AGENT(charge) => {
                    if self.state.players.contains_key(&proxy) && proxy != contractor {
                        Some(Role::GUARD(proxy))
                    } else {
                        Some(Role::SURVIVOR)
                    }
                }
                Role::GUARD(charge) => {
                    if self.state.players.contains_key(&proxy) && proxy != contractor {
                        Some(Role::AGENT(proxy))
                    } else {
                        Some(Role::IDIOT(false))
                    }
                }
                _ => None,
            };
            if let Some(new_role) = new_role {
                updates.push((contractor, new_role));
            }
        }
        for (contractor, new_role) in updates {
            self.refocus(contractor, new_role)?;
        }

        self.event_tx.send(Event::Eliminate { player, role })?;
        self.state.players.remove(&player);
        // Check for end of game
        if let Some(winner) = self.check_end() {
            self.end(winner)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn refocus(&mut self, player: PID, role: Role<PID>) -> Result<(), CoreError<PID>> {
        let former_role = Self::validate_player(&self.state.players, player)?;
        self.state.players.insert(player, role);
        self.state
            .role_history
            .entry(player)
            .or_insert(Vec::new())
            .push(role);
        self.event_tx.send(Event::Refocus {
            player,
            role,
            former_role,
        })?;
        Ok(())
    }

    fn to_day(&mut self, blocks: Option<HashMap<PID, Vec<PID>>>) -> Result<(), CoreError<PID>> {
        self.state.day_no += 1;
        let blocks = blocks.unwrap_or(HashMap::new());
        self.state.phase = Phase::Day {
            votes: HashMap::new(),
            blocks,
            elect_timer: None,
        };
        self.event_tx.send(Event::Day {
            day_no: self.state.day_no,
        })?;
        Ok(())
    }

    fn to_night(&mut self) -> Result<(), CoreError<PID>> {
        self.state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            dawn_timer: None,
        };
        self.event_tx.send(Event::Night {
            day_no: self.state.day_no,
        })?;
        Ok(())
    }

    fn to_eclipse(
        &mut self,
        avenger: PID,
        hammer: PID,
        options: Vec<PID>,
    ) -> Result<(), CoreError<PID>> {
        self.state.phase = Phase::Eclipse {
            avenger,
            hammer,
            options: options.clone(),
        };
        self.event_tx.send(Event::Eclipse {
            avenger,
            hammer,
            options,
        })?;
        Ok(())
    }

    fn validate_player(
        players: &HashMap<PID, Role<PID>>,
        player: PID,
    ) -> Result<Role<PID>, CoreError<PID>> {
        let Some(&role) = players.get(&player) else {
            return Err(CoreError::InvalidPlayer { player });
        };
        Ok(role)
    }

    fn check_dawn(&mut self) -> Result<bool, CoreError<PID>> {
        let Phase::Night {
            targets,
            scheme,
            dawn_timer,
        } = &mut self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Night;
            return Err(CoreError::InvalidPhase { actual, expected });
        };
        if let None = scheme {
            return Ok(false);
        }
        // Check that every targeting role has a target
        for (player, role) in &self.state.players {
            if role.is_targeting() {
                if !targets.contains_key(player) {
                    return Ok(false);
                }
            }
        }
        // Schedule dawn!
        let action_tx = self.action_tx.clone();
        *dawn_timer = Some(Timer::new(
            SystemTime::now() + Duration::from_secs(1),
            (),
            Duration::from_millis(100),
            Box::new(move |_| {
                let (tx, rx) = std::sync::mpsc::channel();
                action_tx.send((Action::Dawn, tx)).unwrap();
                rx.recv().unwrap().expect("Dawn failed!");
            }),
        ));

        return Ok(true);
    }

    fn check_election(
        votes: &HashMap<PID, Choice<PID>>,
        n: usize,
        candidate: Choice<PID>,
    ) -> Option<Vec<PID>> {
        let threshold = match candidate {
            Choice::Player(_) => n / 2 + 1,
            Choice::Abstain => (n + 1) / 2,
        };

        // count the votes for each player
        let mut voters: Vec<PID> = Vec::new();
        for (&voter, &choice) in votes {
            if choice == candidate {
                voters.push(voter);
            }
        }
        if voters.len() >= threshold {
            return Some(voters);
        }

        return None;
    }

    fn check_end(&self) -> Option<Team> {
        let n = self.state.players.len();
        let n_mafia = self
            .state
            .players
            .values()
            .filter(|&&role| role.team() == Team::Mafia)
            .count();

        if n_mafia == 0 {
            // Town wins!
            return Some(Team::Town);
        } else if n - n_mafia <= n_mafia {
            // Mafia wins!
            return Some(Team::Mafia);
        }
        return None;
    }

    fn end(&mut self, winner: Team) -> Result<(), CoreError<PID>> {
        self.state.phase = Phase::End { winner };
        self.event_tx.send(Event::End {
            winner,
            role_history: self.state.role_history.clone(),
        })?;
        Ok(())
    }
}
