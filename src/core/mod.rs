// don't warn about unused imports
#![allow(dead_code, unused_variables, unused_imports)]

pub mod base;
pub mod interface;
pub mod roles;
pub mod timer;

use base::{Choice, ID};
use interface::{
    Action, ActionInput, ActionOutput, CoreError, Event, EventOutput, RespInput, RespOutput,
};
use roles::{DawnState, DawnStateChange, Role, RoleKind, Team};
use timer::Timer;

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
// use std::sync::mpsc::{Receiver, RecvError, SendError, Sender};
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};

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
        elect_timer: Option<Timer>,
    },
    Night {
        targets: HashMap<PID, Choice<PID>>, // actor -> target
        scheme: Option<(PID, Choice<PID>)>, // actor -> (target, choice)
        dawn_timer: Option<Timer>,
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
    pub id: GID,
    state: State<PID>,
    rules: Rules,
    event_tx: EventOutput<PID>,
    action_rx: ActionOutput<PID, ()>,
    action_tx: ActionInput<PID, ()>,
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
impl<PID: ID, GID: ID> Core<PID, GID> {
    pub fn new(
        id: GID,
        players: HashMap<PID, Role<PID>>,
        rules: Rules,
        event_tx: EventOutput<PID>,
        action_rx: ActionOutput<PID, ()>,
        action_tx: ActionInput<PID, ()>,
    ) -> Self {
        let day_no = 0;

        let state = State::new(players);
        Core {
            id,
            state,
            rules,
            event_tx,
            action_rx,
            action_tx,
        }
    }

    pub async fn new_spawned(
        id: GID,
        players: HashMap<PID, Role<PID>>,
        rules: Rules,
        event_tx: EventOutput<PID>,
        action_rx: ActionOutput<PID, ()>,
        action_tx: ActionInput<PID, ()>,
    ) -> JoinHandle<()> {
        let core = Core::new(id, players, rules, event_tx, action_rx, action_tx);
        core.spawn().await
    }

    pub async fn spawn(self) -> JoinHandle<()> {
        let join = tokio::spawn(async move {
            {
                self.run().await;
            }
        });
        join
    }

    pub async fn run(mut self) {
        let mut quit = false;
        while !quit {
            tokio::time::sleep(Duration::from_millis(1)).await;

            let Some((action, response)) = self.action_rx.recv().await else {
                break; // Action Channel closed, quit
            };
            // Do action (non-blocking sync function?)
            let resp = self.handle_action(action).await;
            // If this fails, resp is returned... What should we do in that case? TODO
            if let Err(CoreError::Close) = resp {
                quit = true;
                self.event_tx.send(Event::Close).await.unwrap();
            }
            response.send(resp).expect("Response channel error: {:?}");
        }

        self.event_tx.send(Event::Close).await.unwrap();
    }

    pub async fn send_action(
        action_tx: &ActionInput<PID, ()>,
        action: Action<PID>,
    ) -> Result<(), CoreError<PID>> {
        let (tx, rx) = oneshot::channel();

        action_tx.send((action, tx)).await.expect("Action to send");

        let resp = rx.await.expect("Response to receive");
        resp
    }

    async fn handle_action(&mut self, action: Action<PID>) -> Result<(), CoreError<PID>> {
        let result = match action {
            Action::Start => self.start().await,
            Action::Vote { voter, choice } => self.vote(voter, Some(choice)).await,
            Action::Unvote { voter } => self.vote(voter, None).await,
            Action::Reveal { player } => self.reveal(player).await,
            Action::Target { actor, target } => self.target(actor, target).await,
            Action::Scheme { actor, mark } => self.scheme(actor, mark).await,
            Action::Avenge { avenger, victim } => self.avenge(avenger, victim).await,
            Action::Elect { candidate, hammer } => self.elect(candidate, hammer).await,
            Action::Dawn => self.dawn().await,
            Action::Close => {
                self.event_tx.send(Event::Close).await?;
                Err(CoreError::Close)
            }
        };
        result
    }

    async fn start(&mut self) -> Result<(), CoreError<PID>> {
        self.event_tx.send(Event::Start {}).await?;

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
            self.to_night().await?;
        } else {
            self.to_day(None).await?;
        }
        Ok(())
    }

    async fn vote(
        &mut self,
        voter: PID,
        ballot: Option<Choice<PID>>,
    ) -> Result<(), CoreError<PID>> {
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

        self.event_tx
            .send(Event::Vote {
                voter,
                ballot,
                former_ballot,
            })
            .await?;

        // Check for a former election and whether it still has quorum
        if let Some(choice) = former_ballot {
            if let None = Self::check_election(votes, n, choice) {
                if let Some(timer) = elect_timer {
                    // Cancel election timer
                    timer.cancel().await;
                    *elect_timer = None;
                }
            }
        }

        // Check for a new election
        if let Some(candidate) = ballot {
            if let Some(_) = Self::check_election(votes, n, candidate) {
                // Set election timer
                let action_tx = self.action_tx.clone();
                let t = Timer::new(
                    tokio::time::Instant::now() + Duration::from_secs(1),
                    Duration::from_millis(100),
                    async move {
                        let (tx, rx) = oneshot::channel();
                        action_tx
                            .send((
                                Action::Elect {
                                    candidate: candidate.clone(),
                                    hammer: voter.clone(),
                                },
                                tx,
                            ))
                            .await
                            .unwrap();
                        let resp = rx.await.expect("Election resp failed!");
                    },
                );
                *elect_timer = Some(t);
            }
        }

        Ok(())
    }

    async fn reveal(&self, player: PID) -> Result<(), CoreError<PID>> {
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
                .send(Event::EvidentBlock { blocked, blockers })
                .await?;
            return Ok(());
        }

        self.event_tx.send(Event::Reveal { player, role }).await?;
        Ok(())
    }

    async fn target(&mut self, actor: PID, target: Choice<PID>) -> Result<(), CoreError<PID>> {
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
        self.event_tx.send(Event::Target { actor, target }).await?;

        // TODO: if actor was STRIPPER, make sure they can't kill

        self.check_dawn().await?;
        Ok(())
    }

    async fn scheme(&mut self, actor: PID, mark: Choice<PID>) -> Result<(), CoreError<PID>> {
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
        self.event_tx.send(Event::Scheme { actor, mark }).await?;

        // TODO: if killer was STRIPPER, make sure they can't target

        self.check_dawn().await?;
        Ok(())
    }

    async fn elect(&mut self, candidate: Choice<PID>, hammer: PID) -> Result<(), CoreError<PID>> {
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
            self.event_tx
                .send(Event::Election {
                    choice: candidate,
                    voters: voters.clone(),
                })
                .await?;

            if role.kind() == RoleKind::IDIOT {
                // Go to ECLIPSE
                self.to_eclipse(player, hammer, voters).await?;
                return Ok(());
            }

            if self.eliminate(player, hammer).await? {
                // Game Over!
                return Ok(());
            }
        }
        self.to_night().await?;
        Ok(())
    }

    async fn dawn(&mut self) -> Result<(), CoreError<PID>> {
        let Phase::Night {
            targets, scheme, ..
        } = &self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Night;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        self.event_tx.send(Event::Dawn).await?;

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

        self.perform_night_actions(early_night_actions, &mut dawn_state)
            .await?;

        self.perform_scheme(scheme, &mut dawn_state).await?;

        // Perform Kills
        if dawn_state.killed.len() > 0 {
            let mut game_over = false;
            for (&mark, &killer) in &dawn_state.killed {
                game_over = self.eliminate(mark, killer).await? | game_over;
            }
            if game_over {
                return Ok(());
            }
        } else {
            self.event_tx.send(Event::NoNightKill).await?;
        }

        self.perform_night_actions(late_night_actions, &mut dawn_state)
            .await?;

        self.to_day(Some(dawn_state.blocks)).await?;
        Ok(())
    }

    // Note: Night actions are performed in batches. All actions of a given
    //   priority create their changes at once, then all changes are applied at once.
    async fn perform_night_actions(
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
                changes.extend(
                    a.role
                        .night_action(a.actor, a.target, &dawn_state, &self.event_tx)
                        .await?,
                );
                next = actions.peek();
            }
            dawn_state.apply_changes(changes);
        }
        Ok(())
    }

    async fn perform_scheme(
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
                        self.event_tx
                            .send(Event::EvidentBlock {
                                blocked: savior,
                                blockers: blockers.clone(),
                            })
                            .await?;
                        continue;
                    }
                    saved = true;
                    self.event_tx
                        .send(Event::EvidentSave { savior, mark })
                        .await?;
                }
            }
            if !saved {
                dawn_state.killed.insert(mark, killer);
                self.event_tx.send(Event::Kill { killer, mark }).await?;
            }
        }
        Ok(())
    }

    async fn avenge(&mut self, avenger: PID, victim: Choice<PID>) -> Result<(), CoreError<PID>> {
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

        self.event_tx
            .send(Event::Avenge { avenger, target })
            .await?;

        self.eliminate(target, avenger).await?;

        // change IDIOT's role to win state
        self.refocus(avenger, Role::IDIOT(true)).await?;

        self.eliminate(avenger, hammer).await?;

        self.to_night().await?;
        Ok(())
    }

    async fn eliminate(&mut self, player: PID, proxy: PID) -> Result<bool, CoreError<PID>> {
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
            self.refocus(contractor, new_role).await?;
        }

        self.event_tx
            .send(Event::Eliminate { player, role })
            .await?;
        self.state.players.remove(&player);
        // Check for end of game
        if let Some(winner) = self.check_end() {
            self.end(winner).await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn refocus(&mut self, player: PID, role: Role<PID>) -> Result<(), CoreError<PID>> {
        let former_role = Self::validate_player(&self.state.players, player)?;
        self.state.players.insert(player, role);
        self.state
            .role_history
            .entry(player)
            .or_insert(Vec::new())
            .push(role);
        self.event_tx
            .send(Event::Refocus {
                player,
                role,
                former_role,
            })
            .await?;
        Ok(())
    }

    async fn to_day(
        &mut self,
        blocks: Option<HashMap<PID, Vec<PID>>>,
    ) -> Result<(), CoreError<PID>> {
        self.state.day_no += 1;
        let blocks = blocks.unwrap_or(HashMap::new());
        self.state.phase = Phase::Day {
            votes: HashMap::new(),
            blocks,
            elect_timer: None,
        };
        self.event_tx
            .send(Event::Day {
                day_no: self.state.day_no,
            })
            .await?;
        Ok(())
    }

    async fn to_night(&mut self) -> Result<(), CoreError<PID>> {
        self.state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            dawn_timer: None,
        };
        self.event_tx
            .send(Event::Night {
                day_no: self.state.day_no,
            })
            .await?;
        Ok(())
    }

    async fn to_eclipse(
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
        self.event_tx
            .send(Event::Eclipse {
                avenger,
                hammer,
                options,
            })
            .await?;
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

    async fn check_dawn(&mut self) -> Result<bool, CoreError<PID>> {
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
        let t = Timer::new(
            Instant::now() + Duration::from_secs(1),
            Duration::from_millis(100),
            async move {
                let (tx, rx) = oneshot::channel();
                action_tx
                    .send((Action::Dawn, tx))
                    .await
                    .expect("Action to send");
                let _ = rx.await.expect("Dawn Resp to recv");
            },
        );
        *dawn_timer = Some(t);

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

    async fn end(&mut self, winner: Team) -> Result<(), CoreError<PID>> {
        self.state.phase = Phase::End { winner };
        self.event_tx
            .send(Event::End {
                winner,
                role_history: self.state.role_history.clone(),
            })
            .await?;
        // self.event_tx.send(Event::Close).await?; // TODO: don't do this here?
        Ok(())
    }
}

#[cfg(test)]

mod test {

    use super::*;

    use std::env;
    use tokio::join;

    async fn start_print_event_handler(
        mut event_rx: mpsc::Receiver<Event<u32>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let event = event_rx.recv().await.expect("Event to receive");
                println!("Eprint!: {:?}", event);
                tokio::time::sleep(Duration::from_millis(10)).await;
                if let Event::Close { .. } = event {
                    break;
                }
            }
        })
    }

    async fn end(action_tx: &ActionInput<u32, ()>, joins: Vec<JoinHandle<()>>) {
        let resp = Core::<u32, u32>::send_action(&action_tx, Action::Close).await;
        assert_eq!(resp, Err(CoreError::Close));
        for join in joins {
            join.await.expect("Join to finish");
        }
    }

    #[tokio::test]
    async fn test_core_basic() {
        // Create a basic mafia game

        env::set_var("RUST_BACKTRACE", "1");

        impl ID for u32 {}

        let mut players = HashMap::new();
        players.insert(1, Role::TOWN);
        players.insert(2, Role::TOWN);
        players.insert(3, Role::MAFIA);

        let (event_tx, event_rx) = mpsc::channel::<Event<u32>>(100);

        let event_join = start_print_event_handler(event_rx).await;

        let (action_tx, action_rx) = mpsc::channel(100);

        let core_join =
            Core::new_spawned(0, players, Rules {}, event_tx, action_rx, action_tx.clone()).await;

        let resp = Core::<u32, u32>::send_action(&action_tx, Action::Start).await;
        assert_eq!(resp, Ok(()));

        tokio::time::sleep(Duration::from_millis(100)).await;

        let resp = Core::<u32, u32>::send_action(
            &action_tx,
            Action::Vote {
                voter: 1,
                choice: Choice::Abstain,
            },
        )
        .await;
        assert_eq!(resp, Ok(()));

        let resp = Core::<u32, u32>::send_action(
            &action_tx,
            Action::Vote {
                voter: 2,
                choice: Choice::Abstain,
            },
        )
        .await;
        assert_eq!(resp, Ok(()));

        tokio::time::sleep(Duration::from_millis(2000)).await;

        let resp = Core::<u32, u32>::send_action(&action_tx, Action::Close).await;
        assert_eq!(resp, Err(CoreError::Close));
        for join in vec![core_join, event_join] {
            join.await.expect("Join to finish");
        }
    }
}
