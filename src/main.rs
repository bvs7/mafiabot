mod controller;
mod core;
mod server;

use crate::core::*;

#[macro_use]
extern crate enum_kinds;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let token = std::fs::read_to_string("data/.discord.token").expect("Unable to read file");

    std::env::set_var("DISCORD_TOKEN", token);

    server::start().await;
    Ok(())
}
