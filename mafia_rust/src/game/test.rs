use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use super::comm::*;
use super::player::*;
use super::*;

impl RawPID for u64 {}
impl Source for String {}

#[test]

fn live() {
    assert!(true);
}

#[test]
fn run_basic() {
    let players = vec![
        Player::new(1u64, "p1", Role::TOWN),
        Player::new(2, "p2", Role::COP),
        Player::new(3, "p3", Role::DOCTOR),
        Player::new(4, "p4", Role::CELEB),
        Player::new(5, "p5", Role::MILLER),
        Player::new(6, "p6", Role::MASON),
        Player::new(7, "p7", Role::MAFIA),
        Player::new(8, "p8", Role::GODFATHER),
        Player::new(9, "p9", Role::STRIPPER),
        Player::new(10, "p10", Role::GOON),
        Player::new(11, "p11", Role::IDIOT),
        Player::new(12, "p12", Role::SURVIVOR),
        Player::new(13, "p13", Role::GUARD(0)), // p1
        Player::new(14, "p14", Role::AGENT(1)), // p2
    ];

    let (tx, game_rx) = channel::<Request<u64, String>>();
    let (game_tx, rx) = channel::<Response<u64, String>>();

    let mut game = Game::new(players, game_rx, game_tx);

    let game_thread = game.start();

    thread::sleep(Duration::from_millis(500));

    tx.send(Request {
        cmd: Command::Action(Actor::Player(2), Target::Player(7)),
        src: "action".to_string(),
    });

    dbg!(rx.recv().unwrap());
    dbg!(rx.recv().unwrap());
    dbg!(rx.recv().unwrap());
    dbg!(rx.recv().unwrap());
}
