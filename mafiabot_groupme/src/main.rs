#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashSet;

use groupme::api;

// mod commands;
mod controller;
mod game;
mod lobby;
mod prelude;

mod persistent_state;

const TEST_LOBBY_CHAT_ID: groupme::GroupId = groupme::GroupId(105412553);
const BRIAN_UID: groupme::UserId = groupme::UserId(21642197);

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::init();
    // let lobbies = Vec::from([TEST_LOBBY_CHAT_ID]);
    // let admins = HashSet::from_iter([BRIAN_UID]);
    // let (controller_task, controller_handle) =
    //     controller::Controller::create(lobbies, admins).await;
    // controller_task.await.expect("Controller should run");
}
