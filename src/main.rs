mod core;

// use std::collections::HashMap;

// use crate::core::base::*;
// use crate::core::interface::*;
// use crate::core::roles::*;
use crate::core::*;

#[macro_use]
extern crate enum_kinds;

// use std::thread;
// use std::time;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // println!("Hello, world!");
    // test3().unwrap();
    Ok(())
}
/*
fn test1() -> Result<(), ()> {
    let mut players = HashMap::new();
    players.insert(1, Role::TOWN);
    players.insert(2, Role::TOWN);
    players.insert(3, Role::MAFIA);

    let (event_tx, event_rx) = std::sync::mpsc::channel::<Event<u32>>();
    let (action_tx, action_rx) = std::sync::mpsc::channel();

    let core = Core::new(
        0,
        players,
        Rules {},
        event_tx.clone(),
        action_rx,
        action_tx.clone(),
    );

    let event_reader = thread::spawn(move || loop {
        let event = event_rx.recv().expect("Event to receive");
        println!("{:?}", event);
        if let Event::Close { .. } = event {
            break;
        }
    });

    core.start_thread();

    let (resp_tx, resp_rx) = std::sync::mpsc::channel();

    let vote = Action::Vote {
        voter: 1,
        choice: Choice::Player(3),
    };

    action_tx
        .send((vote, resp_tx.clone()))
        .expect("Action to send");

    let resp = resp_rx.recv().expect("Response to receive");

    println!("{:?}", resp);

    let vote = Action::Vote {
        voter: 2,
        choice: Choice::Player(3),
    };

    action_tx
        .send((vote, resp_tx.clone()))
        .expect("Action to send");

    thread::sleep(time::Duration::from_secs(2));

    action_tx
        .send((Action::Close, resp_tx.clone()))
        .expect("Action to send");

    event_reader.join().expect("Event reader to join");
    Ok(())
}

fn test2() -> Result<(), ()> {
    let mut players = HashMap::new();
    players.insert(1, Role::TOWN);
    players.insert(2, Role::TOWN);
    players.insert(3, Role::MAFIA);
    players.insert(4, Role::TOWN);

    let (event_tx, event_rx) = std::sync::mpsc::channel::<Event<u32>>();
    let (action_tx, action_rx) = std::sync::mpsc::channel();

    let core = Core::new(
        0,
        players,
        Rules {},
        event_tx.clone(),
        action_rx,
        action_tx.clone(),
    );

    let event_reader = thread::spawn(move || loop {
        let event = event_rx.recv().expect("Event to receive");
        println!("{:?}", event);
        if let Event::Close { .. } = event {
            break;
        }
    });

    core.start_thread();

    let (resp_tx, _resp_rx) = std::sync::mpsc::channel();

    let scheme = Action::Scheme {
        actor: 3,
        mark: Choice::Player(1),
    };

    action_tx.send((scheme, resp_tx.clone())).unwrap();

    thread::sleep(time::Duration::from_secs(2));

    println!("Vote 1!");

    let vote = Action::Vote {
        voter: 4,
        choice: Choice::Player(2),
    };

    action_tx.send((vote, resp_tx.clone())).unwrap();

    let vote = Action::Vote {
        voter: 2,
        choice: Choice::Player(2),
    };

    action_tx.send((vote, resp_tx.clone())).unwrap();

    thread::sleep(time::Duration::from_secs(2));

    action_tx
        .send((Action::Close, resp_tx.clone()))
        .expect("Action to send");

    event_reader.join().expect("Event reader to join");
    Ok(())
}

fn test3() -> Result<(), ()> {
    let mut players = HashMap::new();
    players.insert(1, Role::COP);
    players.insert(2, Role::DOCTOR);
    players.insert(3, Role::STRIPPER);
    players.insert(4, Role::CELEB);

    let (event_tx, event_rx) = std::sync::mpsc::channel::<Event<u32>>();
    let (action_tx, action_rx) = std::sync::mpsc::channel();

    let core = Core::new(
        0,
        players,
        Rules {},
        event_tx.clone(),
        action_rx,
        action_tx.clone(),
    );

    let event_reader = thread::spawn(move || loop {
        let event = event_rx.recv().expect("Event to receive");
        println!("{:?}", event);
        if let Event::Close { .. } = event {
            break;
        }
    });

    core.start_thread();

    let (resp_tx, resp_rx) = std::sync::mpsc::channel();

    let send = |action: Action<u32>| {
        // print!("Action: {:?}: ", action);
        action_tx.send((action, resp_tx.clone())).unwrap();
        let resp = resp_rx.recv().unwrap();
        if let Err(e) = resp {
            println!("Error: {:?}", e);
        }
        // println!("{:?}", resp_rx.recv().unwrap());
    };
    send(Action::Vote {
        voter: 0,
        choice: Choice::Player(2),
    });
    send(Action::Reveal { player: 4 });

    send(Action::Target {
        actor: 1,
        target: Choice::Player(3),
    });

    send(Action::Scheme {
        actor: 3,
        mark: Choice::Player(1),
    });

    send(Action::Target {
        actor: 2,
        target: Choice::Player(1),
    });
    send(Action::Target {
        actor: 3,
        target: Choice::Player(4),
    });

    thread::sleep(time::Duration::from_secs(2));

    send(Action::Reveal { player: 4 });

    thread::sleep(time::Duration::from_secs(2));

    action_tx
        .send((Action::Close, resp_tx.clone()))
        .expect("Action to send");

    event_reader.join().expect("Event reader to join");
    Ok(())
}
*/
