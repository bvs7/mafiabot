mod core;

use std::collections::HashMap;

use crate::core::base::*;
use crate::core::error::*;
use crate::core::events::*;
use crate::core::roles::*;
use crate::core::*;

#[macro_use]
extern crate enum_kinds;

use std::thread;
use std::time;

impl ID for u32 {}

fn main() -> Result<(), CoreError<u32>> {
    // println!("Hello, world!");

    let mut players = HashMap::new();
    players.insert(1, Role::TOWN);
    players.insert(2, Role::TOWN);
    players.insert(3, Role::MAFIA);

    let (event_tx, event_rx) = std::sync::mpsc::channel::<Event<u32>>();
    let (action_tx, action_rx) = std::sync::mpsc::channel();

    let mut core = Core::new(
        0,
        players,
        Rules {},
        event_tx.clone(),
        action_rx,
        action_tx.clone(),
    );

    core.start_thread();

    let (resp_tx, resp_rx) = std::sync::mpsc::channel();

    let vote = Action::Vote {
        voter: 1,
        choice: Choice::Player(2),
    };

    action_tx
        .send((vote, resp_tx.clone()))
        .expect("Action to send");

    let mut resp = resp_rx.recv().expect("Response to receive");

    println!("{:?}", resp);

    let mut event = event_rx.try_recv();

    let vote = Action::Vote {
        voter: 3,
        choice: Choice::Player(2),
    };

    action_tx
        .send((vote, resp_tx.clone()))
        .expect("Action to send");

    thread::sleep(time::Duration::from_secs(11));

    while event.is_ok() {
        println!("{:?}", event);
        event = event_rx.try_recv();
    }

    // let mut core = Core::new(0, players, Rules {}, tx);
    // println!("{:#?}", core);
    // println!("{:?}", core.vote(1, Some(Choice::Player(2))));
    // println!("{:?}", core.vote(2, Some(Choice::Player(1))));
    // println!("{:#?}", core);
    Ok(())
}
