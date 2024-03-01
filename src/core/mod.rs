// don't warn about unused imports
#![allow(dead_code)]
// #![allow(dead_code, unused_variables, unused_imports)]

pub mod base;
pub mod interface;
pub mod roles;
pub mod rules;
pub mod timer;

use base::{Choice, ID};
use interface::{
    Action, Command, CommandTx, CoreError, Event, EventRx, Interface, SerializeGameError,
    SerializedGame,
};
use roles::{DawnState, DawnStateChange, NightAction, Role, RoleKind, Team};
use rules::Rules;
use timer::Timer;

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{BinaryHeap, HashMap};
use std::fmt::Debug;
use toml;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Duration;

// Maintains historical data about the game
// Used for revealing information about the game
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(EnumKind, Debug, Clone, Serialize, Deserialize)]
#[enum_kind(PhaseKind, derive(Serialize, Deserialize))]
pub enum Phase<PID: ID> {
    Init,
    Day {
        votes: HashMap<PID, Choice<PID>>, // voter -> choice
        blocks: HashMap<PID, Vec<PID>>,   // blocked -> blockers
    },
    Night {
        targets: HashMap<PID, Choice<PID>>, // actor -> target
        scheme: Option<(PID, Choice<PID>)>, // actor -> (target, choice)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State<PID: ID> {
    pub day_no: u32,
    pub players: HashMap<PID, Role<PID>>,
    pub phase: Phase<PID>,
    pub timer: Option<Timer<PID>>,
    pub role_history: HashMap<PID, Vec<Role<PID>>>,
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
            timer: None,
            role_history,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Core<PID: ID, GID: ID> {
    pub id: GID,
    state: State<PID>,
    rules: Rules,
    #[serde(skip)]
    pub inter: Interface<PID>,
    // event_tx: EventTx<PID>,
    // command_rx: CommandRx<PID>,
    // cmd_tx: CommandTx<PID>,
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
    ) -> (Self, EventRx<PID>, CommandTx<PID>) {
        let day_no = 0;

        let state = State::new(players);

        let (inter, event_rx, cmd_tx) = Interface::new_with_channels();

        let core = Core {
            id,
            state,
            rules,
            inter,
        };
        return (core, event_rx, cmd_tx);
    }

    pub async fn new_spawned(
        id: GID,
        players: HashMap<PID, Role<PID>>,
        rules: Rules,
    ) -> (JoinHandle<()>, EventRx<PID>, CommandTx<PID>) {
        let (core, event_rx, cmd_tx) = Core::new(id, players, rules);
        (core.spawn().await, event_rx, cmd_tx)
    }

    pub async fn spawn(self) -> JoinHandle<()> {
        let join = tokio::spawn(async move {
            {
                self.run().await;
            }
        });
        join
    }

    fn cancel_timers(&mut self) {
        self.state.timer = None;
    }

    pub async fn run(mut self) {
        let mut quit = false;
        while !quit {
            // TODO: do we need this?
            tokio::time::sleep(Duration::from_millis(1)).await;

            // Check timer and perform action if required
            let timer_action = match &self.state.timer {
                Some(timer) => timer.check().await,
                None => None,
            };
            if let Some(action) = timer_action {
                let _ = self.handle_action(action).await;
                self.state.timer = None;
            }

            if self.try_handle_command().await {
                quit = true;
            }
        }

        self.cancel_timers();

        self.inter.send(Event::Close).await.unwrap();
    }

    fn get_serialized_game(&self) -> Result<SerializedGame, SerializeGameError> {
        let state_json = serde_json::to_string_pretty(&self.state)?;
        let rules_toml = toml::to_string_pretty(&self.rules)?;
        Ok(SerializedGame {
            id: self.id.to_string(),
            state: state_json,
            rules: rules_toml,
        })
    }

    async fn try_handle_command(&mut self) -> bool {
        match self.inter.cmd_rx.try_recv() {
            Ok(Command::Action(action, response)) => {
                let resp = self.handle_action(action).await;
                response.send(resp).expect("Response channel error: {:?}");
            }
            Ok(Command::Status(response)) => {
                response
                    .send(Ok(self.state.clone()))
                    .expect("Response channel error: {:?}");
            }
            Ok(Command::Rules(response)) => {
                response
                    .send(Ok(self.rules.clone()))
                    .expect("Response channel error: {:?}");
            }
            Ok(Command::Serialize(response)) => {
                let state_json = serde_json::to_string_pretty(&self.state);
                let rules_toml = toml::to_string_pretty(&self.rules);
                let saved_game = SerializedGame {
                    id: self.id.to_string(),
                    state: state_json.unwrap(),
                    rules: rules_toml.unwrap(),
                };
                response
                    .send(Ok(saved_game))
                    .expect("Response channel error: {:?}");
            }

            Ok(Command::Close) => {
                return true;
            }
            Err(mpsc::error::TryRecvError::Empty) => {}
            Err(mpsc::error::TryRecvError::Disconnected) => {
                return true;
            }
        }
        return false;
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
        };
        result
    }

    async fn start(&mut self) -> Result<(), CoreError<PID>> {
        self.inter
            .send(Event::Start {
                players: self.state.players.clone(),
            })
            .await?;

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
        let Phase::Day { votes, .. } = &mut self.state.phase else {
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

        self.inter
            .send(Event::Vote {
                voter,
                ballot,
                former_ballot,
            })
            .await?;

        if let Some((candidate, hammer, _)) = self.check_election(voter, ballot, former_ballot) {
            self.inter
                .send(Event::ElectionImminent { candidate, hammer })
                .await?;
        }
        Ok(())
    }

    fn check_election(
        &mut self,
        hammer: PID,
        ballot: Option<Choice<PID>>,
        former_ballot: Option<Choice<PID>>,
    ) -> Option<(Choice<PID>, PID, Vec<PID>)> {
        let Phase::Day { votes, .. } = &self.state.phase else {
            return None;
        };
        // Check if previous election is cancelled
        if let Some(former_candidate) = former_ballot {
            match self.state.timer {
                Some(Timer {
                    data: Action::Elect { candidate, .. },
                    ..
                }) if candidate == former_candidate => {
                    if let None = Self::check_quorum(votes, former_candidate) {
                        self.state.timer = None;
                    }
                }
                _ => {}
            }
        }

        if let Some(candidate) = ballot {
            // Check if new election is imminent
            if let Some(voters) = Self::check_quorum(votes, candidate) {
                // Set election timer (if not already set)
                if let None = self.state.timer {
                    let duration = self.rules.timer_rules.election_imminent_time;
                    let end_time = chrono::offset::Local::now() + duration;
                    self.state.timer = Some(Timer {
                        end_time,
                        data: Action::Elect { candidate, hammer },
                    });
                    return Some((candidate, hammer, voters));
                }
            }
        }
        None
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
            self.inter
                .send(Event::EvidentBlock { blocked, blockers })
                .await?;
            return Ok(());
        }

        self.inter.send(Event::Reveal { player, role }).await?;
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

        // Check for Stripper Overload
        if role.kind() == RoleKind::STRIPPER && target != Choice::Abstain {
            if let Some((killer, mark)) = scheme {
                if *killer == actor && *mark != Choice::Abstain {
                    return Err(CoreError::StripperOverload { actor });
                }
            }
        }

        targets.insert(actor, target);
        self.inter.send(Event::Target { actor, target }).await?;

        self.check_dawn()?;
        Ok(())
    }

    async fn scheme(&mut self, actor: PID, mark: Choice<PID>) -> Result<(), CoreError<PID>> {
        let role = Self::validate_player(&self.state.players, actor)?;
        if !role.is_scheming() {
            let role = role.kind();
            return Err(CoreError::ExpectedSchemingRole { role });
        }
        if let Choice::Player(player) = mark {
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

        // Check for Stripper Overload
        if role.kind() == RoleKind::STRIPPER && mark != Choice::Abstain {
            if let Some(target) = targets.get(&actor) {
                if *target != Choice::Abstain {
                    return Err(CoreError::StripperOverload { actor });
                }
            }
        }

        scheme.replace((actor, mark));
        self.inter.send(Event::Scheme { actor, mark }).await?;

        self.check_dawn()?;
        Ok(())
    }

    fn check_dawn(&mut self) -> Result<bool, CoreError<PID>> {
        let Phase::Night { targets, scheme } = &mut self.state.phase else {
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
        if let None = self.state.timer {
            let duration = self.rules.timer_rules.dawn_imminent_time;
            let end_time = chrono::offset::Local::now() + duration;
            self.state.timer = Some(Timer {
                end_time,
                data: Action::Dawn,
            });
        }

        return Ok(true);
    }

    async fn elect(&mut self, candidate: Choice<PID>, hammer: PID) -> Result<(), CoreError<PID>> {
        // Ensure the phase is Day
        let Phase::Day { votes, .. } = &mut self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        if let Choice::Player(player) = candidate {
            let _ = Self::validate_player(&self.state.players, player)?;
        }

        let Some(voters) = Self::check_quorum(votes, candidate) else {
            return Err(CoreError::ExpectedElection { candidate });
        };

        self.inter
            .send(Event::Election {
                candidate,
                hammer,
                voters: voters.clone(),
            })
            .await?;

        if let Choice::Player(player) = candidate {
            let role = Self::validate_player(&self.state.players, player)?;

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

    fn collect_night_actions(
        players: &HashMap<PID, Role<PID>>,
        targets: &HashMap<PID, Choice<PID>>,
    ) -> Result<(BinaryHeap<NightAction<PID>>, BinaryHeap<NightAction<PID>>), CoreError<PID>> {
        let mut early_actions = BinaryHeap::new();
        let mut late_actions = BinaryHeap::new();
        for (&actor, &target) in targets.iter() {
            let Choice::Player(target) = target else {
                continue;
            };
            let role = Core::<PID, GID>::validate_player(players, actor)?;
            let _ = Core::<PID, GID>::validate_player(players, target)?;
            let Some(priority) = role.night_action_priority() else {
                return Err(CoreError::ExpectedTargetingRole { role: role.kind() });
            };
            let action = NightAction {
                actor,
                role,
                target,
                priority,
            };
            if priority < 0 {
                early_actions.push(action);
            } else {
                late_actions.push(action);
            }
        }
        Ok((early_actions, late_actions))
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

        self.inter.send(Event::Dawn).await?;

        let (early_night_actions, late_night_actions) =
            Self::collect_night_actions(&self.state.players, targets)?;

        let mut dawn_state: DawnState<PID> = DawnState {
            blocks: HashMap::new(),
            saves: HashMap::new(),
            killed: HashMap::new(),
        };

        self.perform_night_actions(early_night_actions, &mut dawn_state)
            .await?;

        self.perform_scheme(scheme, &mut dawn_state).await?;

        // Perform Kills (first killer does the kill, but end of game isn't checked until all kills are performed)
        if dawn_state.killed.len() > 0 {
            let mut eliminations = Vec::new();
            for (&mark, killers) in &dawn_state.killed {
                eliminations.push((mark, killers.first().unwrap().clone()));
            }
            if self.eliminate_many(eliminations).await? {
                return Ok(());
            }
        } else {
            self.inter.send(Event::NoNightKill).await?;
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
                let action = actions.pop().expect("Checked for some above!");
                let new_changes = action.perform(dawn_state, &self.inter.event_tx).await?;
                changes.extend(new_changes);
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
        let changes = NightAction::perform_scheme(scheme, dawn_state, &self.inter.event_tx).await?;
        dawn_state.apply_changes(changes);
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

        self.inter.send(Event::Avenge { avenger, target }).await?;

        // change IDIOT's role to win state
        self.refocus(avenger, Role::IDIOT(true)).await?;

        self.eliminate_many(vec![(target, avenger), (avenger, hammer)])
            .await?;

        self.to_night().await?;
        Ok(())
    }

    async fn eliminate(&mut self, player: PID, proxy: PID) -> Result<bool, CoreError<PID>> {
        self.eliminate_many(vec![(player, proxy)]).await
    }

    async fn eliminate_many(
        &mut self,
        eliminations: Vec<(PID, PID)>,
    ) -> Result<bool, CoreError<PID>> {
        for (player, proxy) in eliminations {
            let role = Self::validate_player(&self.state.players, player)?;

            self.check_refocus(player, proxy).await?;

            self.inter.send(Event::Eliminate { player, role }).await?;
            self.state.players.remove(&player);
        }
        // Check for end of game
        if let Some(winner) = self.check_end() {
            self.end(winner).await?;
            return Ok(true);
        }
        Ok(false)
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
        self.inter
            .send(Event::End {
                winner,
                alive: self.state.players.iter().map(|(k, _)| *k).collect(),
                role_history: self.state.role_history.clone(),
            })
            .await?;
        // self.inter.send(Event::Close).await?; // TODO: don't do this here?
        Ok(())
    }

    async fn check_refocus(&mut self, player: PID, proxy: PID) -> Result<(), CoreError<PID>> {
        // Check contracting roles
        let mut updates: Vec<(PID, Role<PID>)> = Vec::new();
        for (&contractor, &role) in &self.state.players {
            if let Some(charge) = role.contract() {
                if charge == player {
                    let new_role = match role {
                        Role::AGENT(_) => {
                            if self.state.players.contains_key(&proxy) && proxy != contractor {
                                Some(Role::GUARD(proxy))
                            } else {
                                Some(Role::SURVIVOR)
                            }
                        }
                        Role::GUARD(_) => {
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
            }
        }
        for (contractor, new_role) in updates {
            self.refocus(contractor, new_role).await?;
        }
        Ok(())
    }

    async fn refocus(&mut self, player: PID, role: Role<PID>) -> Result<(), CoreError<PID>> {
        let former_role = Self::validate_player(&self.state.players, player)?;
        self.state.players.insert(player, role);
        self.state
            .role_history
            .entry(player)
            .or_insert(Vec::new())
            .push(role);
        self.inter
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
        };
        self.state.timer = None;
        self.inter
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
        };
        self.state.timer = None;
        self.inter
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
        self.state.timer = None;
        self.inter
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

    fn check_quorum(votes: &HashMap<PID, Choice<PID>>, candidate: Choice<PID>) -> Option<Vec<PID>> {
        let n = votes.len();
        let threshold = match candidate {
            Choice::Player(_) => n / 2 + 1,
            Choice::Abstain => (n + 1) / 2,
        };

        let voters: Vec<PID> = votes
            .iter()
            .filter_map(|(&v, &ch)| (ch == candidate).then_some(v))
            .collect();

        if voters.len() >= threshold {
            return Some(voters);
        } else {
            return None;
        }
    }
}

#[cfg(test)]

mod test {

    use super::*;
    use tokio::join;

    impl ID for u32 {}

    async fn start_print_event_handler(
        mut event_rx: mpsc::Receiver<Event<u32>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let event = event_rx.recv().await.expect("Event to receive");
                println!("EVENT: {:?}", event);
                tokio::time::sleep(Duration::from_millis(10)).await;
                if let Event::Close { .. } = event {
                    break;
                }
            }
        })
    }

    fn get_players(n: u8) -> HashMap<u32, Role<u32>> {
        let mut players = HashMap::new();
        let role_list = vec![
            Role::TOWN,         // 1
            Role::TOWN,         // 2
            Role::MAFIA,        // 3
            Role::COP,          // 4
            Role::DOCTOR,       // 5
            Role::STRIPPER,     // 6
            Role::CELEB,        // 7
            Role::IDIOT(false), // 8
            Role::SURVIVOR,     // 9
            Role::AGENT(1),     // 10
            Role::GUARD(1),     // 11
        ];
        for i in 1..=n {
            players.insert(i as u32, role_list[i as usize - 1]);
        }
        players
    }

    async fn vote(
        cmd_tx: &CommandTx<u32>,
        voter: u32,
        choice: Choice<u32>,
    ) -> Result<(), CoreError<u32>> {
        let action = Action::Vote { voter, choice };
        Interface::send_action(&cmd_tx, action).await
    }

    async fn votes(
        cmd_tx: &CommandTx<u32>,
        voters: Vec<u32>,
        choice: Choice<u32>,
    ) -> Result<(), CoreError<u32>> {
        for voter in voters {
            vote(&cmd_tx, voter, choice).await?;
        }
        Ok(())
    }

    async fn target(
        cmd_tx: &CommandTx<u32>,
        actor: u32,
        target: Choice<u32>,
    ) -> Result<(), CoreError<u32>> {
        let action = Action::Target { actor, target };
        Interface::send_action(&cmd_tx, action).await
    }

    async fn scheme(
        cmd_tx: &CommandTx<u32>,
        actor: u32,
        mark: Choice<u32>,
    ) -> Result<(), CoreError<u32>> {
        let action = Action::Scheme { actor, mark };
        Interface::send_action(&cmd_tx, action).await
    }

    async fn wait() {
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    #[tokio::test]
    async fn test_targeting() -> Result<(), CoreError<u32>> {
        // env::set_var("RUST_BACKTRACE", "1");

        let players = get_players(7);

        let (core_join, event_rx, cmd_tx) = Core::new_spawned(0, players, Rules::test()).await;

        let event_handler_join = start_print_event_handler(event_rx).await;

        Interface::send_action(&cmd_tx, Action::Start).await?;

        let state = Interface::send_status(&cmd_tx).await?;
        // println!("{:#?}", state);

        assert_eq!(state.day_no, 1);
        assert_eq!(state.players.len(), 7);
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        vote(&cmd_tx, 1, Choice::Player(3)).await?;
        vote(&cmd_tx, 2, Choice::Player(3)).await?;
        vote(&cmd_tx, 3, Choice::Player(3)).await?;
        vote(&cmd_tx, 4, Choice::Player(3)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        // println!("{:#?}", state);
        assert_eq!(state.day_no, 1);
        assert_eq!(state.players.len(), 6);
        assert_eq!(state.phase.kind(), PhaseKind::Night);

        scheme(&cmd_tx, 6, Choice::Player(1)).await?;

        assert_eq!(
            target(&cmd_tx, 6, Choice::Player(4)).await,
            Err(CoreError::StripperOverload { actor: 6 })
        );

        scheme(&cmd_tx, 6, Choice::Abstain).await?;
        target(&cmd_tx, 6, Choice::Player(4)).await?;

        target(&cmd_tx, 4, Choice::Player(1)).await?;
        target(&cmd_tx, 5, Choice::Player(5)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;

        // println!("{:#?}", state);

        assert_eq!(state.day_no, 2);
        assert_eq!(state.players.len(), 6);
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        vote(&cmd_tx, 7, Choice::Player(1)).await?;
        vote(&cmd_tx, 6, Choice::Player(1)).await?;
        vote(&cmd_tx, 1, Choice::Player(1)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        vote(&cmd_tx, 1, Choice::Abstain).await?;
        vote(&cmd_tx, 2, Choice::Abstain).await?;
        assert_eq!(
            vote(&cmd_tx, 3, Choice::Abstain).await,
            Err(CoreError::InvalidPlayer { player: 3 })
        );
        vote(&cmd_tx, 4, Choice::Abstain).await?;

        Interface::send_action(&cmd_tx, Action::Unvote { voter: 4 }).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        vote(&cmd_tx, 7, Choice::Abstain).await?;
        vote(&cmd_tx, 6, Choice::Abstain).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Night);

        target(&cmd_tx, 4, Choice::Player(6)).await?;
        target(&cmd_tx, 5, Choice::Player(5)).await?;

        target(&cmd_tx, 6, Choice::Abstain).await?;
        scheme(&cmd_tx, 6, Choice::Player(1)).await?;

        target(&cmd_tx, 4, Choice::Player(1)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        vote(&cmd_tx, 7, Choice::Player(2)).await?;
        vote(&cmd_tx, 6, Choice::Player(2)).await?;
        vote(&cmd_tx, 2, Choice::Player(2)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;

        //4-COP, 5-DOCTOR, 6-STRIPPER, 7-CELEB
        assert_eq!(state.phase.kind(), PhaseKind::Night);
        assert_eq!(state.players.len(), 4);

        scheme(&cmd_tx, 6, Choice::Abstain).await?;
        target(&cmd_tx, 6, Choice::Player(4)).await?;
        target(&cmd_tx, 4, Choice::Player(5)).await?;
        target(&cmd_tx, 5, Choice::Player(4)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        vote(&cmd_tx, 7, Choice::Abstain).await?;
        vote(&cmd_tx, 6, Choice::Abstain).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Night);

        target(&cmd_tx, 6, Choice::Player(7)).await?;
        target(&cmd_tx, 4, Choice::Player(7)).await?;
        target(&cmd_tx, 5, Choice::Player(5)).await?;
        scheme(&cmd_tx, 6, Choice::Abstain).await?;

        wait().await;

        Interface::send_action(&cmd_tx, Action::Reveal { player: 7 }).await?;

        vote(&cmd_tx, 7, Choice::Player(6)).await?;
        vote(&cmd_tx, 5, Choice::Player(6)).await?;
        vote(&cmd_tx, 4, Choice::Player(6)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::End);

        assert!(matches!(state.phase, Phase::End { winner: Team::Town }));

        Interface::send_close(&cmd_tx).await;

        let _ = join!(core_join, event_handler_join);

        return Ok(());
    }

    #[tokio::test]
    async fn test_contracts() -> Result<(), CoreError<u32>> {
        // 1-TOWN, 2-TOWN, 3-MAFIA, 4-COP, 5-DOCTOR, 6-STRIPPER,
        // 7-CELEB, 8-IDIOT, 9-SURVIVOR, 10-AGENT(1), 11-GUARD(1)

        let players = get_players(11);
        let (core_join, event_rx, cmd_tx) = Core::new_spawned(0, players, Rules::test()).await;
        let event_handler_join = start_print_event_handler(event_rx).await;

        Interface::send_action(&cmd_tx, Action::Start).await?;

        votes(&cmd_tx, vec![2, 3, 4, 5, 6, 7, 8], Choice::Player(1)).await?;

        votes(&cmd_tx, vec![2, 7], Choice::Abstain).await?;

        wait().await;

        vote(&cmd_tx, 7, Choice::Player(1)).await?;
        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Night);
        assert!(matches!(state.players[&10], Role::GUARD(7)));
        assert!(matches!(state.players[&11], Role::AGENT(7)));

        target(&cmd_tx, 4, Choice::Player(3)).await?;
        target(&cmd_tx, 5, Choice::Player(5)).await?;
        scheme(&cmd_tx, 3, Choice::Player(2)).await?;
        target(&cmd_tx, 6, Choice::Player(10)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Day);

        votes(&cmd_tx, vec![3, 4, 5, 6, 7], Choice::Player(8)).await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        assert_eq!(state.phase.kind(), PhaseKind::Eclipse);

        Interface::send_action(
            &cmd_tx,
            Action::Avenge {
                avenger: 8,
                victim: Choice::Player(7),
            },
        )
        .await?;

        wait().await;

        let state = Interface::send_status(&cmd_tx).await?;
        println!("{:#?}", state);
        assert_eq!(state.phase.kind(), PhaseKind::Night);

        Interface::send_close(&cmd_tx).await;

        let _ = join!(core_join, event_handler_join);

        Ok(())
    }

    #[tokio::test]
    async fn test_serialize() -> Result<(), CoreError<u32>> {
        // Setup a game (in the middle of Election Imminent state)

        let players = get_players(11);
        let (core_join, event_rx, cmd_tx) = Core::new_spawned(0, players, Rules::test()).await;
        let event_handler_join = start_print_event_handler(event_rx).await;

        Interface::send_action(&cmd_tx, Action::Start).await?;

        votes(&cmd_tx, vec![2, 3, 4, 5, 6, 7, 8], Choice::Player(1)).await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to serialize the game state

        let (tx, rx) = oneshot::channel();
        cmd_tx.send(Command::Serialize(tx)).await.unwrap();
        let saved_game = rx
            .await
            .expect("Response channel error: {:?}")
            .expect("Serialization Error: {:?}");

        Interface::send_close(&cmd_tx).await;

        println!("\nGame ID:\n---\n{}", saved_game.id);
        println!("\nGame State:\n---\n{}", saved_game.state);
        println!("\nGame Rules:\n---\n{}", saved_game.rules);

        let _ = join!(core_join, event_handler_join);

        Ok(())
    }
}
