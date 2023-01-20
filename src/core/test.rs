// Basic Game 1

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use super::*;

// Create a basic game, that, when started will go to Day Phase (because odd number of players)
fn create_basic_game_1() -> (Game<u64>, Receiver<Event<u64>>) {
    // Players for a simple 5 player game
    let players = vec![
        Player::new(101, Role::TOWN),
        Player::new(102, Role::COP),
        Player::new(103, Role::DOCTOR),
        Player::new(104, Role::MAFIA),
        Player::new(105, Role::TOWN),
    ];
    // No contract roles
    let contracts = Vec::new();

    // Set up Comm output
    let (tx, rx): (Sender<Event<u64>>, Receiver<Event<u64>>) = mpsc::channel();

    let game = Game::new(1, players, contracts, Comm::new(&tx));
    return (game, rx);
}

// Create a basic game that will start in Night Phase (because even number of players)
fn create_basic_game_2() -> (Game<u64>, Receiver<Event<u64>>) {
    // Players for a simple 4 player game
    let players = vec![
        Player::new(101, Role::TOWN),
        Player::new(102, Role::COP),
        Player::new(103, Role::DOCTOR),
        Player::new(104, Role::MAFIA),
    ];
    // No contract roles
    let contracts = Vec::new();

    // Set up Comm output
    let (tx, rx): (Sender<Event<u64>>, Receiver<Event<u64>>) = mpsc::channel();

    let game = Game::new(1, players, contracts, Comm::new(&tx));
    return (game, rx);
}

fn expect_eventkind(rx: &Receiver<Event<u64>>, kind: EventKind) {
    let event = rx.try_recv();

    if let Err(e) = event {
        assert!(false, "TryRecvError: {:?}", e);
    }
    let event = event.unwrap();

    let event = event;
    assert_eq!(event.kind(), kind);
}

#[test]
fn invalid_votes() {
    let (mut game, rx) = create_basic_game_1();

    assert!(game.start().is_ok());
    // Read off event queue and expect Day Phase
    expect_eventkind(&rx, EventKind::Init);
    expect_eventkind(&rx, EventKind::Start);
    expect_eventkind(&rx, EventKind::Day);

    assert!(
        game.handle(Action::Vote {
            voter: 404,
            ballot: None
        })
        .is_err(),
        "Invalid voter, should fail"
    );

    assert!(
        game.handle(Action::Vote {
            voter: 101,
            ballot: Some(Choice::Player(404))
        })
        .is_err(),
        "Invalid ballot target, should fail"
    );

    let (mut game, rx) = create_basic_game_2();
    assert!(game.start().is_ok());
    expect_eventkind(&rx, EventKind::Init);
    expect_eventkind(&rx, EventKind::Start);
    expect_eventkind(&rx, EventKind::Night);

    assert!(
        game.handle(Action::Vote {
            voter: 101,
            ballot: Some(Choice::Player(102))
        })
        .is_err(),
        "Invalid phase, should fail"
    );
}
