// Trying to make some top down stuff here
#![allow(dead_code)]

use std::{collections::HashMap, hash::Hash, sync::Arc};

use axum::{
    debug_handler,
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Local};
use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};
use serde_json;
use tokio::sync::{mpsc, oneshot, Notify, RwLock};

trait Domain {
    fn domain() -> impl Iterator<Item = Self>;
}

#[macro_use]
extern crate enum_kinds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind, Serialize)]
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
}

impl Domain for RoleKind {
    fn domain() -> impl Iterator<Item = Self> {
        use RoleKind::*;
        return vec![TOWN, COP, DOCTOR, MAFIA].into_iter();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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

impl Domain for Team {
    fn domain() -> impl Iterator<Item = Self> {
        use Team::*;
        return vec![Town, Mafia, Rogue].into_iter();
    }
}

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

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        todo!();
    }
}

#[derive(Clone, Copy, Deserialize)]
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

#[derive(Debug)]
struct PlayerLog(HashMap<u64, (bool, Vec<Role>)>);

impl PlayerLog {
    fn new(role_assignments: HashMap<u64, Role>) -> Self {
        PlayerLog(
            role_assignments
                .into_iter()
                .map(|(pid, role)| (pid, (true, vec![role])))
                .collect(),
        )
    }

    /// Living players and their roles
    fn players(&self) -> HashMap<u64, Role> {
        self.0
            .iter()
            .filter_map(|(&pid, (alive, role_log))| {
                alive.then_some((pid, *role_log.last().expect("At least one role")))
            })
            .collect()
    }

    fn get_role_assignments(&self) -> HashMap<u64, Role> {
        self.0
            .iter()
            .map(|(&pid, (_, log))| (pid, *log.first().expect("At least one role")))
            .collect()
    }

    fn log_new_role(&mut self, pid: u64, new_role: Role) {
        let (_, log) = self.0.get_mut(&pid).expect("fn should get valid pid");
        log.push(new_role);
    }

    fn eliminate(&mut self, pid: u64) -> Result<(), ()> {
        let (ref mut alive, _) = self.0.get_mut(&pid).expect("fn should get valid pid");
        if *alive {
            *alive = false;
            Ok(())
        } else {
            Err(())
        }
    }

    fn amts_from_roles_domain<T>(&self) -> HashMap<T, u32>
    where
        T: From<Role> + Domain + Hash + Eq,
    {
        let mut result = self.amts_roles_that(T::from);
        for item in T::domain() {
            if !result.contains_key(&item) {
                result.insert(item, 0);
            }
        }
        return result;
    }

    // Have a "domain" for types?
    fn amts_from_roles<T>(&self) -> HashMap<T, u32>
    where
        T: From<Role> + Hash + Eq,
    {
        self.amts_roles_that(T::from)
    }

    fn amt_roles_that<P>(&self, cond: P) -> u32
    where
        P: Fn(Role) -> bool,
    {
        *self.amts_roles_that(cond).get(&true).unwrap_or(&0)
    }

    fn amts_roles_that<T, P>(&self, cond: P) -> HashMap<T, u32>
    where
        T: Hash + Eq,
        P: Fn(Role) -> T,
    {
        let mut result = HashMap::<T, u32>::new();
        let _ = self.players().iter().map(|(_, &role)| {
            let entry = result.entry(cond(role)).or_default();
            *entry += 1;
        });
        return result;
    }
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, EnumKind)]
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

#[derive(Debug)]
struct Rules {}

#[derive(Debug)]
struct Core {
    game_id: u64,
    day_no: u32,
    player_log: PlayerLog,
    phase: Phase,
    rules: Rules,
    event_log: Vec<Event>,
    timer: Timer,
}

// fn serialize_core_privilege()

// How to send Core object to users?

// We will want some various "privileges":

// Town player
// Mafia player
// Admin
// Observation

// Rules:
//   Known Role amts
//   Known Team amts
//   Known Mafia amt
//   Known Player amt

// What gets sent?
/*
- game_id
- day_no
- phase data:
    - Day: votes
    - Night: ...
    - End: winner
- PlayerLog: Map<pid, Option<Role>> can work (null otherwise)
    - For Admin, this can be full?
    - For Town, this is just players -> null
    - For Mafia, this is just players but known teammates have Role given...
- Counts (based on rules, get amts of roles/teams/etc) This is based on public knowledge
    - Either: Mapping one following to u32 amt:
        - RoleKind
        - Team
        - "mafia"/"not mafia"
        - "players"
- Rules (Are these always needed? Or should there be state vs metadata?)
- Event Log
- Timer

What kinds of reads do we have?
- events
    - EventLog
- state
    - Day_no
    - Phase Data
    - Playerlog
    - Counts
- meta-info
    - Game_id
    - Rules
    - Role Assignments (starting players if not priveleged)

It would be nice to just implement serialize with different privilege levels...
*/

impl Core {
    fn players(&self) -> HashMap<u64, Role> {
        self.player_log.players()
    }

    async fn handle_action(&mut self, action: Action) {
        todo!()
    }

    fn validate_action(&self, action: &Action) -> Result<(), Error> {
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
- events
    - EventLog
- state
    - Day_no
    - Phase Data
    - Playerlog
    - Counts
- meta-info
    - Game_id
    - Rules
    - Role Assignments (starting players if not priveleged)
*/

enum Privilege {
    Town,
    Mafia,
    Admin,
    Observer,
}

type ActionResponder = oneshot::Sender<Result<(), Error>>;
type ActionSender = mpsc::Sender<(Action, ActionResponder)>;
type ActionReceiver = mpsc::Receiver<(Action, ActionResponder)>;

type AppState = Arc<RwLock<Core>>;

async fn get_game_status(
    action_input: ActionSender,
    State(state): State<AppState>,
) -> &'static str {
    "Get Game Status"
}

async fn post_action(
    action_input: ActionSender,
    state: Arc<RwLock<Core>>,
    action: Action,
) -> &'static str {
    let (responder, response) = oneshot::channel();
    action_input
        .send((action, responder))
        .await
        .expect("Action Send");
    let resp = response.await.expect("Action Response");
    "Post Action"
}

// Serve api.
async fn run_api(action_input: ActionSender, core: Arc<RwLock<Core>>) -> Result<(), ()> {
    // let (action_sender, mut action_queue) = mpsc::channel(10);
    let action_input_1 = action_input.clone();
    let action_input_2 = action_input.clone();

    let get_game_status = |state: State<AppState>| get_game_status(action_input_1, state);

    let post_action = |State(state): State<AppState>, Json(action): Json<Action>| {
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

    todo!();
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
    let timer_notify = read_core.timer.notify.clone();
    loop {
        tokio::select! {
            a = action_queue.recv() => handle_action(&core, a).await,
            _ = timer_notify.notified() => handle_timer(&core).await,
        }
    }

    Ok(())
}

/* Stuff we need in top level:
- Set up axum and serve
  - Needs reference to RwLock? Part of state
  - Also needs ActionQueue Sender

*/

#[tokio::main]
async fn main() -> Result<(), ()> {
    // TODO: spawn game loop task and axum serve task

    Ok(())
}
