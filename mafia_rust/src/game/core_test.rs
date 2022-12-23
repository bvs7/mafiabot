// use std::sync::mpsc::Receiver;

// use super::comm::*;
// use super::player::*;
// use super::*;

// impl RawPID for u64 {}
// impl Source for String {}

// struct DontFindHandler {
//     events: Vec<EventKind>,
// }

// #[allow(dead_code)]
// fn delay_milis(milis: u64) {
//     std::thread::sleep(std::time::Duration::from_millis(milis));
// }

// #[allow(dead_code)]
// fn print_recv(rx: &mut Receiver<Response<u64, String>>) {
//     delay_milis(100);
//     loop {
//         match rx.try_recv() {
//             Ok(resp) => println!("Received: {:?}", resp),
//             Err(_) => break,
//         }
//     }
// }

// #[allow(dead_code)]
// fn empty_resp(rx: &mut Receiver<Response<u64, String>>) {
//     loop {
//         match rx.try_recv() {
//             Ok(_) => (),
//             Err(_) => break,
//         }
//     }
// }

// #[allow(dead_code)]
// fn resp_handle(rx: &mut Receiver<Response<u64, String>>, eh: &mut impl EventHandler<u64, String>) {
//     loop {
//         match rx.try_recv() {
//             Ok(resp) => eh.handle(resp.event, resp.src),
//             Err(_) => break,
//         }
//     }
// }

// #[test]
// fn test_dawn() -> Result<(), super::Error<u64>> {
//     let players = vec![
//         Player::new(100, "p0", Role::TOWN),
//         Player::new(1u64, "p1", Role::COP),
//         Player::new(2, "p2", Role::COP),
//         Player::new(3, "p3", Role::COP),
//         Player::new(4, "p4", Role::COP),
//         Player::new(5, "p5", Role::DOCTOR),
//         Player::new(6, "p6", Role::DOCTOR),
//         Player::new(7, "p7", Role::DOCTOR),
//         Player::new(8, "p8", Role::DOCTOR),
//         Player::new(9, "p9", Role::STRIPPER),
//         Player::new(10, "p10", Role::STRIPPER),
//         Player::new(11, "p11", Role::STRIPPER),
//         Player::new(12, "p12", Role::STRIPPER),
//         Player::new(13, "p13", Role::MAFIA),
//         Player::new(14, "p14", Role::CELEB),
//         Player::new(15, "p15", Role::CELEB),
//         Player::new(16, "p16", Role::CELEB),
//     ];

//     let (tx, game_rx) = std::sync::mpsc::channel();
//     let (game_tx, mut rx) = std::sync::mpsc::channel();

//     let comm = Comm::<u64, String>::new(game_rx, game_tx);

//     let mut game = Game::new(players, comm);

//     game.phase = Phase::new_night(1);
//     game.handle(Command::Target {
//         actor: 1,
//         target: Choice::Player(1),
//     })?;
//     game.handle(Command::Target {
//         actor: 2,
//         target: Choice::Player(5),
//     })?;
//     game.handle(Command::Target {
//         actor: 3,
//         target: Choice::Player(9),
//     })?;
//     game.handle(Command::Target {
//         actor: 4,
//         target: Choice::Abstain,
//     })?;
//     game.handle(Command::Target {
//         actor: 5,
//         target: Choice::Player(5),
//     })?;
//     game.handle(Command::Target {
//         actor: 6,
//         target: Choice::Player(1),
//     })?;
//     game.handle(Command::Target {
//         actor: 7,
//         target: Choice::Player(7),
//     })?;
//     game.handle(Command::Target {
//         actor: 8,
//         target: Choice::Player(9),
//     })?;
//     game.handle(Command::Target {
//         actor: 9,
//         target: Choice::Player(2),
//     })?;
//     game.handle(Command::Target {
//         actor: 10,
//         target: Choice::Player(2),
//     })?;
//     game.handle(Command::Target {
//         actor: 11,
//         target: Choice::Player(13),
//     })?;
//     game.handle(Command::Target {
//         actor: 12,
//         target: Choice::Player(14),
//     })?;
//     game.handle(Command::Mark {
//         killer: 13,
//         mark: Choice::Player(100),
//     })?;

//     game.handle(Command::Reveal { celeb: 14 })?;

//     let mut eh = DisplayEventHandler::new();

//     resp_handle(&mut rx, &mut eh);

//     Ok(())
// }
