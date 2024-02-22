mod core;

use std::collections::HashMap;

use crate::core::base::*;
use crate::core::error::*;
use crate::core::events::*;
use crate::core::roles::*;
use crate::core::*;

#[macro_use]
extern crate enum_kinds;

fn main() -> Result<(), CoreError<u32>> {
    println!("Hello, world!");

    let mut players = HashMap::new();
    players.insert(1, Role::TOWN);
    players.insert(2, Role::TOWN);
    players.insert(3, Role::MAFIA);

    let (tx, _rx) = std::sync::mpsc::channel::<Event<u32>>();

    let mut core = Core::new(0, players, Rules {}, tx);
    println!("{:#?}", core);
    println!("{:?}", core.vote(1, Some(Choice::Player(2))));
    println!("{:?}", core.vote(2, Some(Choice::Player(1))));
    println!("{:#?}", core);
    Ok(())
}
