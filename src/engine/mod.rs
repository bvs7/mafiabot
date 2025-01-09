use std::{
    collections::HashMap,
    fmt::Pointer,
    hash::Hash,
    io::{Read, Write},
    ops::Deref,
    sync::Arc,
};

use chrono::{DateTime, Local};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use tokio::{
    sync::{broadcast, mpsc, oneshot, watch, Notify, RwLock, RwLockReadGuard},
    task::JoinHandle,
};
use tracing::{self, debug, error, event, info, warn};

pub use role::{Role, RoleKind, Team};
mod role {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind, Serialize, Deserialize)]
    #[enum_kind(RoleKind, derive(Hash, Serialize))]
    pub enum Role {
        TOWN,
        COP,
        DOCTOR,
        MAFIA,
    }

    impl Role {
        pub fn is_targeting(&self) -> bool {
            use Role::*;
            match self {
                COP | DOCTOR => true,
                TOWN | MAFIA => false,
            }
        }
        pub fn is_scheming(&self) -> bool {
            use Role::*;
            match self {
                MAFIA => true,
                TOWN | COP | DOCTOR => false,
            }
        }
        pub fn team(&self) -> Team {
            return Team::from(*self);
        }
    }

    impl PartialEq<Role> for RoleKind {
        fn eq(&self, role: &Role) -> bool {
            let role_kind: RoleKind = role.into();
            self == &role_kind
        }
    }

    impl std::fmt::Display for RoleKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum Team {
        Town,
        Mafia,
        Rogue,
    }

    impl From<Role> for Team {
        fn from(role: Role) -> Self {
            use Role::*;
            match role {
                TOWN | COP | DOCTOR => Self::Town,
                MAFIA => Self::Mafia,
            }
        }
    }
}

pub use interface::{Action, ActionMsg, ActionRx, ActionTx, Error, Event, EventRx, EventTx};
mod interface {
    use super::{state::PhaseKind, RoleKind};
    use serde::{Deserialize, Serialize};
    use tokio::sync::{broadcast, mpsc, oneshot};

    #[derive(Debug)]
    pub enum Error {
        InvalidPhase {
            expected: PhaseKind,
            actual: PhaseKind,
        },
        InvalidActor {
            pid: u64,
        },
        InvalidOther {
            pid: u64,
        },
        ExpectedTargetingRole {
            actual: RoleKind,
        },
        ExpectedSchemingRole {
            actual: RoleKind,
        },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::InvalidPhase { expected, actual } => {
                    write!(f, "Invalid Phase. Expected {expected} but got {actual}")
                }
                Error::InvalidActor { pid } => write!(f, "Invalid actor player id: {pid}"),
                Error::InvalidOther { pid } => write!(f, "Invalid other player id: {pid}"),
                Error::ExpectedTargetingRole { actual } => {
                    write!(f, "Expected targing role, got {actual}")
                }
                Error::ExpectedSchemingRole { actual } => {
                    write!(f, "Expected scheming role, got {actual}")
                }
            }
        }
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    pub enum Action {
        Start,
        Vote {
            voter: u64,
            ballot: Option<Option<u64>>,
        },
        Target {
            actor: u64,
            choice: Option<u64>,
        },
        Scheme {
            killer: u64,
            mark: Option<u64>,
        },
        Reveal {
            actor: u64,
        },
    }

    impl Action {
        pub fn actor(&self) -> Option<u64> {
            use Action::*;
            match self {
                Vote { voter: actor, .. }
                | Target { actor, .. }
                | Scheme { killer: actor, .. }
                | Reveal { actor } => Some(*actor),
                _ => None,
            }
        }

        pub fn other(&self) -> Option<u64> {
            use Action::*;
            match self {
                Vote {
                    ballot: Some(Some(other)),
                    ..
                }
                | Target {
                    choice: Some(other),
                    ..
                }
                | Scheme {
                    mark: Some(other), ..
                } => Some(*other),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Event {
        Election {
            candidate: Option<u64>, // Choice
            hammer: u64,
            voters: Vec<u64>,
        },
        Dawn, // Potentially note those who failed to do night actions
    }

    pub type ActionMsg = (Action, oneshot::Sender<Result<(), Error>>);
    pub type ActionRx = mpsc::Receiver<ActionMsg>;
    pub type ActionTx = mpsc::Sender<ActionMsg>;
    pub type EventRx = broadcast::Receiver<Event>;
    pub type EventTx = broadcast::Sender<Event>;
}

pub use state::{Phase, PhaseKind, Rules, State};
mod state {
    use core::time;
    use std::{
        collections::HashMap,
        hash::Hash,
        io::{Read, Write},
        ops::Deref,
        sync::Arc,
    };

    use chrono::{DateTime, Local};
    use serde::{ser::SerializeStruct, Deserialize, Serialize};
    use tokio::sync::Notify;
    use tracing::info;

    use super::{Action, Error, Event, EventTx, Role, RoleKind, Team};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct LogEntry {
        alive: bool,
        role: Role,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        log: Vec<Role>,
    }

    impl From<Role> for LogEntry {
        fn from(role: Role) -> Self {
            LogEntry {
                alive: true,
                role,
                log: Vec::new(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct PlayerLog(HashMap<u64, LogEntry>);

    impl Deref for PlayerLog {
        type Target = HashMap<u64, Role>;
        fn deref(&self) -> &Self::Target {
            todo!()
        }
    }

    impl PlayerLog {
        fn new<'a>(registry: impl IntoIterator<Item = &'a (u64, Role)>) -> Self {
            PlayerLog(
                registry
                    .into_iter()
                    .map(|&(pid, role)| (pid, role.into()))
                    .collect(),
            )
        }

        /// Living players and their roles
        fn players(&self) -> HashMap<u64, Role> {
            self.0
                .iter()
                .filter_map(|(&pid, &LogEntry { alive, role, .. })| alive.then_some((pid, role)))
                .collect()
        }

        fn get_role_assignments(&self) -> HashMap<u64, Role> {
            self.0
                .iter()
                .map(|(&pid, LogEntry { role, log, .. })| (pid, *(log.get(0).unwrap_or(role))))
                .collect()
        }

        fn log_new_role(&mut self, pid: u64, new_role: Role) {
            let LogEntry { role, log, .. } = self.0.get_mut(&pid).expect("fn should get valid pid");
            log.push(*role);
            *role = new_role;
        }

        fn eliminate(&mut self, pid: u64) -> Result<(), ()> {
            let LogEntry { alive, .. } = self.0.get_mut(&pid).expect("fn should get valid pid");
            alive.then(|| *alive = false).ok_or(())
        }

        /// Count the number of roles using f(role)
        ///
        /// Examples:
        /// ```
        /// // plog roles: [TOWN;5] + [MAFIA;2] + [IDIOT]
        /// let rolekinds_counts = plog.counts(|r| RoleKind::from(r));
        /// // {RoleKind::TOWN : 5, RoleKind::MAFIA : 2, RoleKind::IDIOT: 1}
        /// let team_counts = plog.counts(|r| Team::from(r));
        /// // {Team::Town: 5, Team::Mafia: 2, Team::Rogue: 1}
        /// let mafia_counts = plog.counts(|r|
        ///     if matches!(Team::from(r), Team::Mafia) {"Mafia"} else {"Not Mafia"});
        /// // {"Not Mafia": 6, "Mafia": 2}
        /// let player_count = plog.counts(|_| "Players");
        /// // {"Players": 8}
        /// ```
        fn counts<T, F>(&self, f: F) -> HashMap<T, u32>
        where
            T: Eq + Hash,
            F: Fn(Role) -> T,
        {
            let mut counts = HashMap::new();
            for (_, role) in self.players() {
                *counts.entry(f(role)).or_insert(0) += 1;
            }
            return counts;
        }

        fn mafia_counter() -> fn(Role) -> &'static str {
            |r| {
                if matches!(r.team(), Team::Mafia) {
                    "Mafia"
                } else {
                    "Not Mafia"
                }
            }
        }

        /// players() but censors based on a player's perspective
        fn censored(&self, role: Role) -> HashMap<u64, Option<Role>> {
            let map = match role {
                m if m.team() == Team::Mafia => |r: Role| (r.team() == Team::Mafia).then_some(r),
                t if t.team() == Team::Town => |_| None,
                _ => |_| None,
            };
            let map = |(pid, r)| (pid, map(r));
            return self.players().into_iter().map(map).collect();
        }
    }

    #[derive(Debug)]
    struct Timer {
        notify: Arc<Notify>, // Arc to allow cloning
        data: Option<(DateTime<Local>, Event)>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, EnumKind)]
    #[enum_kind(PhaseKind)]
    pub enum Phase {
        Init,
        Day {
            votes: HashMap<u64, Option<u64>>, // voter -> ballot
            blocks: HashMap<u64, Vec<u64>>,   // blocked -> blockers
        },
        Night {
            targets: HashMap<u64, Option<u64>>, // actor -> target
            scheme: Option<(u64, Option<u64>)>, // killer -> mark
        },
        End {
            winning_team: Team,
        },
    }

    impl std::fmt::Display for PhaseKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Phase {
        fn censored<'a>(&'a self) -> CensoredPhase<'a> {
            match self {
                Phase::Init => CensoredPhase::Init,
                Phase::Day { votes, .. } => CensoredPhase::Day { votes },
                Phase::Night { .. } => CensoredPhase::Night,
                Phase::End { winning_team } => CensoredPhase::End { winning_team },
            }
        }
    }

    #[derive(Debug, Clone, Serialize)]
    pub enum CensoredPhase<'a> {
        Init,
        Day {
            votes: &'a HashMap<u64, Option<u64>>,
        },
        Night,
        End {
            winning_team: &'a Team,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Rules {}

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct State {
        game_id: u64,
        day_no: u32,
        players: PlayerLog,
        phase: Phase,
        rules: Rules,
        #[serde(skip_serializing_if = "Option::is_none")]
        timer_data: Option<(DateTime<Local>, Event)>,
    }

    impl State {
        pub fn new<'a>(
            game_id: u64,
            registry: impl IntoIterator<Item = &'a (u64, Role)>,
            rules: Rules,
        ) -> Self {
            Self {
                game_id,
                day_no: 0,
                players: PlayerLog::new(registry),
                phase: Phase::Init,
                rules,
                timer_data: None,
            }
        }

        pub fn save<W>(&self, writer: W) -> serde_json::Result<()>
        where
            W: Write,
        {
            serde_json::to_writer(writer, self)
        }

        pub fn load<R>(reader: R) -> serde_json::Result<Self>
        where
            R: Read,
        {
            serde_json::from_reader(reader)
        }

        pub async fn handle_action(&mut self, _action: Action, _event_tx: &EventTx) {}

        pub fn validate_action(&self, action: &Action) -> Result<(), Error> {
            info!("Validating action {:?}", action);
            let expected = match action {
                Action::Start => Some(PhaseKind::Init),
                Action::Vote { .. } | Action::Reveal { .. } => Some(PhaseKind::Day),
                Action::Target { .. } | Action::Scheme { .. } => Some(PhaseKind::Night),
            };
            if let Some(expected) = expected {
                let actual: PhaseKind = (&self.phase).into();
                if expected != actual {
                    return Err(Error::InvalidPhase { actual, expected });
                }
            }

            if let Some(actor) = action.actor() {
                let role = *self
                    .players
                    .players()
                    .get(&actor)
                    .ok_or(Error::InvalidActor { pid: actor })?;

                // if matches!(action, Action::Reveal { .. }) && !matches!(role.kind(), RoleKind::CELEB) {
                //     return Err(InvalidActionError::ExpectedCeleb {
                //         actual: role.kind(),
                //     });
                // }

                if matches!(action, Action::Target { .. }) && !role.is_targeting() {
                    return Err(Error::ExpectedTargetingRole {
                        actual: role.into(),
                    });
                }

                if matches!(action, Action::Scheme { .. }) && !role.is_scheming() {
                    return Err(Error::ExpectedSchemingRole {
                        actual: role.into(),
                    });
                }

                if let Some(other) = action.other() {
                    let _ = *self
                        .players
                        .get(&actor)
                        .ok_or(Error::InvalidOther { pid: other })?;
                }
            }
            Ok(())
        }

        pub async fn handle_timer(&mut self, _event_tx: &EventTx) -> Result<(), ()> {
            todo!()
        }
    }

    impl<'a> State {
        pub fn censor_serialize(&'a self) -> impl Serialize + use<'a> {
            struct SerState<'a> {
                state: &'a State,
            }

            impl<'a> Serialize for SerState<'a> {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let state = self.state;
                    let players = state.players.censored(Role::MAFIA);
                    let counts = state.players.counts(PlayerLog::mafia_counter());

                    let mut n = 4;
                    if self.state.timer_data.is_some() {
                        n += 1;
                    }
                    let mut s = serializer.serialize_struct("State", n)?;
                    s.serialize_field("day_no", &state.day_no)?;
                    s.serialize_field("players", &players)?;
                    s.serialize_field("counts", &counts)?;
                    s.serialize_field("phase", &state.phase)?;
                    if self.state.timer_data.is_some() {
                        s.serialize_field("timer_data", &state.timer_data)?;
                    }
                    s.end()
                }
            }
            SerState { state: &self }
        }
    }

    #[derive(Debug, Clone, Serialize)]
    struct CensoredState<'a, T>
    where
        T: Clone + std::fmt::Debug + Serialize,
    {
        game_id: u64,
        day_no: u32,
        players: HashMap<u64, Option<Role>>,
        counts: HashMap<T, u32>,
        phase: CensoredPhase<'a>,
        rules: &'a Rules,
        #[serde(skip_serializing_if = "Option::is_none")]
        timer_data: &'a Option<(DateTime<Local>, Event)>,
    }

    impl<'a, T> CensoredState<'a, T>
    where
        T: From<Role> + Eq + Hash + Clone + std::fmt::Debug + Serialize,
    {
        pub fn from_state<F>(
            State {
                game_id,
                day_no,
                players,
                phase,
                rules,
                timer_data,
            }: &'a State,
            privilege: Role,
            count_rule: F,
        ) -> Self
        where
            F: Fn(Role) -> T,
        {
            Self {
                game_id: *game_id,
                day_no: *day_no,
                players: players.censored(privilege),
                counts: players.counts(count_rule),
                phase: phase.censored(),
                rules: rules,
                timer_data: timer_data,
            }
        }
    }
}

pub use game::Game;
mod game {
    use std::sync::Arc;

    use tokio::{
        sync::{Notify, RwLock, RwLockReadGuard},
        task::JoinHandle,
    };
    use tracing::{debug, info, warn};

    use super::{ActionMsg, ActionRx, EventTx, Role, Rules, State};

    #[derive(Debug)]
    pub struct Game {
        pub state: Arc<RwLock<State>>,
        timer: Arc<Notify>,
        action_rx: ActionRx,
        event_tx: EventTx,
        quit: Arc<Notify>,
    }

    impl Game {
        pub fn new<'a>(
            game_id: u64,
            registry: impl IntoIterator<Item = &'a (u64, Role)>,
            rules: Rules,
            action_rx: ActionRx,
            event_tx: EventTx,
            quit: Arc<Notify>,
        ) -> Self {
            Self {
                state: Arc::new(RwLock::new(State::new(game_id, registry, rules))),
                timer: Arc::new(Notify::new()),
                action_rx,
                event_tx,
                quit,
            }
        }

        pub async fn handle_action(&self, (action, responder): ActionMsg) {
            debug!("Got Action: {:?}", action);
            let result = {
                let rstate = self.state.read().await;
                rstate.validate_action(&action)
            };
            let valid = result.is_ok();
            responder.send(result).unwrap();
            if valid {
                let mut wstate = self.state.write().await;
                wstate.handle_action(action, &self.event_tx).await;
            }
        }

        pub async fn handle_timer(&self) {
            let mut wstate = self.state.write().await;
            wstate.handle_timer(&self.event_tx).await;
        }

        /// Run the game loop, receiving actions, watching timers
        pub async fn run(mut self) {
            debug!("Starting Game Loop");
            loop {
                debug!("Game Loop Beginning");
                tokio::select! {
                    _ = self.quit.notified() => {
                        info!("Got Quit Notification");
                        break;
                    },
                    a = self.action_rx.recv() => if let Some(act) = a {
                        self.handle_action(act).await
                    } else {
                        warn!("self.action_rx.recv() closed? quitting");
                        break;

                    },
                    _ = self.timer.notified() => self.handle_timer().await,
                }
            }
        }

        /// Spawn a tokio task to run this game, consuming it, then return.
        pub async fn start(self) -> JoinHandle<()> {
            tokio::spawn(self.run())
        }
    }
}
#[cfg(test)]
mod test {
    use std::time::Duration;
    use tracing_test::traced_test;

    use super::*;

    fn basic_game() -> (Game, ActionTx, EventRx, Arc<Notify>) {
        let (a_tx, a_rx) = mpsc::channel(100);
        let (e_tx, e_rx) = broadcast::channel(100);
        let quit = Arc::new(Notify::new());

        let registry: Vec<(u64, Role)> = vec![
            (1, Role::TOWN),
            (2, Role::COP),
            (3, Role::DOCTOR),
            (4, Role::MAFIA),
        ];

        let game = Game::new(0, &registry, Rules {}, a_rx, e_tx, quit.clone());

        return (game, a_tx, e_rx, quit);
    }

    #[tokio::test]
    async fn basic() {
        assert!(true);
    }

    #[traced_test]
    #[tokio::test]
    async fn quit() {
        let (game, a_tx, _, quit) = basic_game();
        // Start game
        let game_join = tokio::spawn(async move {
            debug!("Run Game Thread");
            game.run().await
        });

        drop(a_tx);

        // quit.notify_one();

        tokio::select! {
            _ = game_join => {},
            _ = tokio::time::sleep(Duration::from_millis(10000)) => {panic!("Failed to join game");}
        }
    }
}
