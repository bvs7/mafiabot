// Trying to make some top down stuff here
#![allow(dead_code)]

use std::{collections::HashMap, hash::Hash, sync::Arc};

use axum::{
    debug_handler,
    extract::{Json, State as AppState},
    http::{header, HeaderMap},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Local};
use serde::{de::DeserializeOwned, ser::SerializeStruct, Deserialize, Serialize, Serializer};
use serde_json;
use tokio::sync::{mpsc, oneshot, Notify, RwLock};
use tracing::{self, event, info};
use tracing_subscriber;

#[macro_use]
extern crate enum_kinds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind, Serialize, Deserialize)]
#[enum_kind(RoleKind, derive(Hash, Serialize))]
enum Role {
    TOWN,
    COP,
    DOCTOR,
    MAFIA,
}

impl Role {
    fn is_targeting(&self) -> bool {
        use Role::*;
        matches!(self, COP | DOCTOR)
    }
    fn team(&self) -> Team {
        return Team::from(*self);
    }
}

impl PartialEq<Role> for RoleKind {
    fn eq(&self, role: &Role) -> bool {
        let role_kind: RoleKind = role.into();
        self == &role_kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Team {
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

#[derive(Debug)]
enum Error {
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
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum Action {
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
    fn actor(&self) -> Option<u64> {
        use Action::*;
        match self {
            Vote { voter: actor, .. }
            | Target { actor, .. }
            | Scheme { killer: actor, .. }
            | Reveal { actor } => Some(*actor),
            _ => None,
        }
    }

    fn other(&self) -> Option<u64> {
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

    fn validate(&self, core: &Core) -> Result<(), Error> {
        // let expected = match self {
        //     Action::Start => PhaseKind::Init,
        //     Action::Vote { .. } | Action::Reveal { .. } => PhaseKind::Day,
        //     Action::Target { .. } | Action::Scheme { .. } => PhaseKind::Night,
        // };

        // let actual = core.phase.kind();
        // if expected != actual {
        //     return Err(InvalidActionError::InvalidPhase { actual, expected });
        // }

        if let Some(actor) = self.actor() {
            let role = *core
                .players()
                .get(&actor)
                .ok_or(Error::InvalidActor { pid: actor })?;

            //     if matches!(self, Action::Reveal { .. }) && !matches!(role.kind(), RoleKind::CELEB) {
            //         return Err(InvalidActionError::ExpectedCeleb {
            //             actual: role.kind(),
            //         });
            //     }

            if matches!(self, Action::Target { .. }) && !role.is_targeting() {
                return Err(Error::ExpectedTargetingRole {
                    actual: role.into(),
                });
            }

            //     if matches!(self, Action::Scheme { .. }) && !role.is_scheming() {
            //         return Err(InvalidActionError::ExpectedSchemingRole { role: role.kind() });
            //     }
            // }

            if let Some(other) = self.other() {
                let _ = *core
                    .players()
                    .get(&actor)
                    .ok_or(Error::InvalidOther { pid: other })?;
            }
        }
        Ok(())
    }
}

// right now it's a wrapper... how could it be different?
// What do we want. to do with it?
/*
create from role assignments
check if a player_id is alive
get a living player's current role
show role history after the game ends (in very few cases will it be more than one!)
*/

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

impl PlayerLog {
    fn new(registry: impl IntoIterator<Item = (u64, Role)>) -> Self {
        PlayerLog(
            registry
                .into_iter()
                .map(|(pid, role)| (pid, role.into()))
                .collect(),
        )
    }

    /// Living players and their roles
    fn players(&self) -> HashMap<u64, Role> {
        self.0
            .iter()
            .filter_map(|(&pid, LogEntry { alive, role, .. })| (*alive).then_some((pid, *role)))
            .collect()
    }

    fn get_role_assignments(&self) -> HashMap<u64, Role> {
        self.0
            .iter()
            .map(|(&pid, LogEntry { role, log, .. })| (pid, *(log.get(0).unwrap_or(role))))
            .collect()
    }

    fn log_new_role(&mut self, pid: u64, new_role: Role) {
        let LogEntry { alive, role, log } = self.0.get_mut(&pid).expect("fn should get valid pid");
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

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Event {
    Election {
        candidate: Option<u64>, // Choice
        hammer: u64,
        voters: Vec<u64>,
    },
    Dawn, // Potentially note those who failed to do night actions
}

#[derive(Debug)]
struct Timer {
    notify: Arc<Notify>, // Arc to allow cloning
    data: Option<(DateTime<Local>, Event)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumKind)]
#[enum_kind(PhaseKind)]
enum Phase {
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

#[derive(Debug, Serialize, Deserialize)]
struct Rules {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    day_no: u32,
    #[serde(rename = "players")]
    player_log: PlayerLog,
    phase: Phase,
    #[serde(skip_serializing_if = "Option::is_none")]
    timer_data: Option<(DateTime<Local>, Event)>,
}

// Return a struct that can be serialized with censorship
impl<'a> State {
    fn censor_serialize(&'a self) -> impl Serialize + use<'a> {
        struct SerState<'a> {
            state: &'a State,
        }

        impl<'a> Serialize for SerState<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let state = self.state;
                let players = state.player_log.censored(Role::MAFIA);
                let counts = state.player_log.counts(PlayerLog::mafia_counter());

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

#[derive(Debug, Serialize, Deserialize)]
struct Core {
    game_id: u64,
    state: State,
    rules: Rules,
    events: Vec<Event>,
    #[serde(skip)]
    timer_notify: Arc<Notify>,
}

impl Core {
    fn new(game_id: u64, registry: impl IntoIterator<Item = (u64, Role)>, rules: Rules) -> Self {
        Core {
            game_id,
            state: State {
                day_no: 0,
                player_log: PlayerLog::new(registry),
                phase: Phase::Init,
                timer_data: None,
            },
            rules,
            events: Vec::new(),
            timer_notify: Arc::new(Notify::new()),
        }
    }

    fn players(&self) -> HashMap<u64, Role> {
        self.state.player_log.players()
    }

    async fn handle_action(&mut self, action: Action) {}

    #[tracing::instrument]
    fn validate_action(&self, action: &Action) -> Result<(), Error> {
        info!("{:?}", self);
        let expected = match action {
            Action::Start => Some(PhaseKind::Init),
            Action::Vote { .. } | Action::Reveal { .. } => Some(PhaseKind::Day),
            Action::Target { .. } | Action::Scheme { .. } => Some(PhaseKind::Night),
        };
        if let Some(expected) = expected {
            let actual: PhaseKind = (&self.state.phase).into();
            if expected != actual {
                return Err(Error::InvalidPhase { actual, expected });
            }
        }

        if let Some(actor) = action.actor() {
            let role = *self
                .players()
                .get(&actor)
                .ok_or(Error::InvalidActor { pid: actor })?;

            //     if matches!(self, Action::Reveal { .. }) && !matches!(role.kind(), RoleKind::CELEB) {
            //         return Err(InvalidActionError::ExpectedCeleb {
            //             actual: role.kind(),
            //         });
            //     }

            if matches!(action, Action::Target { .. }) && !role.is_targeting() {
                return Err(Error::ExpectedTargetingRole {
                    actual: role.into(),
                });
            }

            //     if matches!(self, Action::Scheme { .. }) && !role.is_scheming() {
            //         return Err(InvalidActionError::ExpectedSchemingRole { role: role.kind() });
            //     }
            // }

            if let Some(other) = action.other() {
                let _ = *self
                    .players()
                    .get(&actor)
                    .ok_or(Error::InvalidOther { pid: other })?;
            }
        }
        Ok(())
    }
}

/*
API description

Model:
We have the Core, which includes State (everything needed to know about the game) and other handles.

Views:
Full Core. Serialization of the Full Core is used to save the core?

Then state. State includes core.state and counts as well?

*/

type ActionResponder = oneshot::Sender<Result<(), Error>>;
type ActionSender = mpsc::Sender<(Action, ActionResponder)>;
type ActionReceiver = mpsc::Receiver<(Action, ActionResponder)>;

async fn get_game_status(
    action_input: ActionSender,
    AppState(state): AppState<Arc<RwLock<Core>>>,
) -> Json<impl Serialize> {
    // Grab state
    let read_core = state.read().await;
    let st = read_core.state.clone();
    Json(st)
}

async fn post_action(
    action_input: ActionSender,
    state: Arc<RwLock<Core>>,
    action: Action,
) -> Result<(), String> {
    let (responder, response) = oneshot::channel();
    action_input
        .send((action, responder))
        .await
        .expect("Action Send");
    response
        .await
        .expect("Action Response")
        .map_err(|e| format!("{:?}", e))
}

// Serve api.
async fn run_api(action_input: ActionSender, core: Arc<RwLock<Core>>) -> Result<(), ()> {
    // let (action_sender, mut action_queue) = mpsc::channel(10);
    let action_input_1 = action_input.clone();
    let action_input_2 = action_input.clone();

    let get_game_status =
        |state: AppState<Arc<RwLock<Core>>>| get_game_status(action_input_1, state);

    let post_action = |AppState(state): AppState<Arc<RwLock<Core>>>, Json(action): Json<Action>| {
        post_action(action_input_2, state, action)
    };

    let app = Router::new()
        .route("/", get(get_game_status))
        .route("/", post(post_action))
        .with_state(core);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn handle_action(core: &Arc<RwLock<Core>>, a: Option<(Action, ActionResponder)>) {
    if let Some((action, responder)) = a {
        let read_core = core.read().await;
        let result = read_core.validate_action(&action);
        let valid = result.is_ok();
        responder.send(result);

        if valid {
            let mut write_core = core.write().await;
            write_core.handle_action(action).await;
        }
    }
}

async fn handle_timer(core: &Arc<RwLock<Core>>) {
    // Grab core?? Hmmmm
}

// Game loop.
async fn run_game(mut action_queue: ActionReceiver, core: Arc<RwLock<Core>>) -> Result<(), ()> {
    // get notify from core
    let read_core = core.read().await;
    let timer_notify = read_core.timer_notify.clone();
    loop {
        tokio::select! {
            a = action_queue.recv() => handle_action(&core, a).await,
            _ = timer_notify.notified() => handle_timer(&core).await,
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).unwrap();

    info!("Starting");

    let (tx, rx) = mpsc::channel(100);

    let registry = HashMap::from([
        (1, Role::TOWN),
        (2, Role::COP),
        (3, Role::DOCTOR),
        (4, Role::MAFIA),
    ]);
    let core = Arc::new(RwLock::new(Core::new(0, registry, Rules {})));
    let core2 = core.clone();

    let game_task = tokio::spawn(async move { run_game(rx, core).await });

    run_api(tx, core2).await.unwrap();

    Ok(())
}
