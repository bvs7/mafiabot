#[macro_use]
extern crate enum_kinds;

mod engine;
mod groupme;
mod server;

use anyhow;

// Game loop.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}
