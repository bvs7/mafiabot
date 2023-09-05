#![allow(unused)]

mod core;
mod error;
mod prelude;
mod utils;

use crate::prelude::*;

use core::game::*;
use core::*;
use std::collections::HashMap;

fn main() -> MResult<()> {
    let mut players = Players { 0: HashMap::new() };
    players.0.insert(
        0,
        RoleHist {
            role: Role::GUARD(1),
            history: vec![], //[Role::AGENT(1)],
        },
    );

    players.0.insert(
        1,
        RoleHist {
            role: Role::TOWN,
            history: vec![], //[Role::AGENT(1)],
        },
    );

    players.0.insert(
        2,
        RoleHist {
            role: Role::MAFIA,
            history: vec![], //[Role::AGENT(1)],
        },
    );

    let role = Role::GUARD(1);
    let mut names: HashMap<PID, String> = HashMap::new();

    names.insert(0, "zero".to_string());
    names.insert(1, "one".to_string());
    names.insert(2, "two".to_string());
    println!("{:?}", &players);
    let serplayers: SerPlayers = (players, &names).into();
    println!("{}", serde_json::to_string(&serplayers).unwrap());
    let players: Players = (serplayers, &names).into();
    println!("{:?}", players);
    Ok(())
}
