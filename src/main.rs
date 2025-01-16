#[macro_use]
extern crate enum_kinds;

mod engine;
mod groupme;
mod server;

use anyhow;
use tracing::subscriber::set_global_default;

// Game loop.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::debug!("Getting groupme_token");

    groupme::try_websocket_subscribe().await
}
