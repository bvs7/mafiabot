// use std::{collections::HashMap, hash::Hash, sync::Arc};

// use axum::{
//     debug_handler,
//     extract::{Json, State as AppState},
//     http::{header, HeaderMap},
//     response::IntoResponse,
//     routing::{get, post},
//     Router,
// };
// use chrono::{DateTime, Local};
// use serde::{de::DeserializeOwned, ser::SerializeStruct, Deserialize, Serialize, Serializer};
// use serde_json;
// use tokio::sync::{mpsc, oneshot, Notify, RwLock};
// use tracing::{self, event, info};
// use tracing_subscriber;

// use crate::engine::{Action, Error, Game};

// /*
// API description

// Model:
// We have the Core, which includes State (everything needed to know about the game) and other handles.

// Views:
// Full Core. Serialization of the Full Core is used to save the core?

// Then state. State includes core.state and counts as well?

// */
// type ActionResponder = oneshot::Sender<Result<(), Error>>;
// type ActionSender = mpsc::Sender<(Action, ActionResponder)>;
// type ActionReceiver = mpsc::Receiver<(Action, ActionResponder)>;

// async fn get_game_status(
//     action_input: ActionSender,
//     AppState(state): AppState<Arc<RwLock<Core>>>,
// ) -> () {
//     // Grab state
//     let read_core = state.read().await;
//     // let st = read_core.state.clone();
//     // Json(st)
//     todo!()
// }

// async fn post_action(
//     action_input: ActionSender,
//     state: Arc<RwLock<Core>>,
//     action: Action,
// ) -> Result<(), String> {
//     let (responder, response) = oneshot::channel();
//     action_input
//         .send((action, responder))
//         .await
//         .expect("Action Send");
//     response
//         .await
//         .expect("Action Response")
//         .map_err(|e| format!("{:?}", e))
// }

// // Serve api.
// async fn run_api(action_input: ActionSender, core: Arc<RwLock<Core>>) -> Result<(), ()> {
//     // let (action_sender, mut action_queue) = mpsc::channel(10);
//     let action_input_1 = action_input.clone();
//     let action_input_2 = action_input.clone();

//     let get_game_status =
//         |state: AppState<Arc<RwLock<Core>>>| get_game_status(action_input_1, state);

//     let post_action = |AppState(state): AppState<Arc<RwLock<Core>>>, Json(action): Json<Action>| {
//         post_action(action_input_2, state, action)
//     };

//     let app = Router::new()
//         .route("/", get(get_game_status))
//         .route("/", post(post_action))
//         .with_state(core);

//     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
//         .await
//         .unwrap();
//     axum::serve(listener, app).await.unwrap();
//     Ok(())
// }

use crate::engine::Error;
use axum::{http::StatusCode, response::IntoResponse};

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}
