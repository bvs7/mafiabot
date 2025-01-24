#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unreachable_code)]

#[macro_use]
extern crate enum_kinds;

mod engine;
mod groupme;
mod server;

mod refactor;

use anyhow;

// Game loop.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(())
}
