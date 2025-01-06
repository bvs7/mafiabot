// Trying to make some top down stuff here

use std::sync::Arc;

use axum::{
    debug_handler,
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use tokio::sync::{mpsc, oneshot, Notify, RwLock};

#[macro_use]
extern crate enum_kinds;

enum Role {}

enum InvalidActionError {}

impl IntoResponse for InvalidActionError {
    fn into_response(self) -> axum::response::Response {
        todo!();
    }
}

#[derive(Serialize, Deserialize)]
enum Action {
    // Start,
    // Vote {
    //     voter: u64,
    //     ballot: Option<Choice<u64>>,
    // },
    // Target {
    //     actor: u64,
    //     choice: Choice<u64>,
    // },
    // Scheme {
    //     killer: u64,
    //     mark: Choice<u64>,
    // },
    // Reveal {
    //     actor: u64,
    // },
}

impl Action {
    fn validate(&self, core: &Core) -> Result<(), InvalidActionError> {
        todo!()
        // let expected = match self {
        //     Action::Start => PhaseKind::Init,
        //     Action::Vote { .. } | Action::Reveal { .. } => PhaseKind::Day,
        //     Action::Target { .. } | Action::Scheme { .. } => PhaseKind::Night,
        // };

        // let actual = core.phase.kind();
        // if expected != actual {
        //     return Err(InvalidActionError::InvalidPhase { actual, expected });
        // }

        // if let Some(actor) = self.actor() {
        //     let role = core
        //         .validate_player(actor)
        //         .map_err(|err| InvalidActionError::InvalidActor { player: err.pid })?;

        //     if matches!(self, Action::Reveal { .. }) && !matches!(role.kind(), RoleKind::CELEB) {
        //         return Err(InvalidActionError::ExpectedCeleb {
        //             actual: role.kind(),
        //         });
        //     }

        //     if matches!(self, Action::Target { .. }) && !role.is_targeting() {
        //         return Err(InvalidActionError::ExpectedTargetingRole { role: role.kind() });
        //     }

        //     if matches!(self, Action::Scheme { .. }) && !role.is_scheming() {
        //         return Err(InvalidActionError::ExpectedSchemingRole { role: role.kind() });
        //     }
        // }

        // if let Some(other) = self.other() {
        //     let _ = core
        //         .validate_player(other)
        //         .map_err(|err| InvalidActionError::InvalidOther { other: err.pid })?;
        // }

        // Ok(())
    }
}

enum Phase {
    Init,
    Day {
        votes: (),
        blocks: (),
        // impending_election: Option<(Time, ballot)>,
    },
}

/*
Should all of these be under the same RwLock?
Yeah I think so...
Some things are locked in: game_id, rules
it might be nice to access Event Log independently?
We just need all fields of Core to be Send/Sync...
Could put Event log and state separately?
Timer notify is Arc, so that it can be cloned out. Then an internal Core method can call notify on it...
timer can independently check for election? Or check for dawn, etc...
*/

#[derive(Debug)]
struct Core {
    // game_id: u64,
    // day_no: u32,
    // players: HashMap< PID, Role>,
    // phase: Phase,
    // role_history: HashMap<PID, Vec<Role>>,
    // rules: Rules,
    // event_log: Vec<Event>
    timer_notify: Arc<Notify>,
}

impl Core {
    async fn handle_action(&mut self, action: Action) {
        todo!()
    }

    fn validate_action(&self, action: &Action) -> Result<(), InvalidActionError> {
        action.validate(self)
    }

    fn validate_player(&self, player: u64) -> Result<Role, u64> {
        todo!()
    }
}

type ActionResponder = oneshot::Sender<Result<(), InvalidActionError>>;
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
    let timer_notify = read_core.timer_notify.clone();
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
