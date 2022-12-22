// use std::collections::HashMap;
// use std::slice::Iter;
// use std::sync::mpsc;
// use std::sync::mpsc::Receiver;
// use std::sync::mpsc::Sender;
// use std::thread;
// use std::thread::JoinHandle;
// use std::time::Duration;

// use super::comm::*;
// use super::player::*;
// use super::*;

// impl RawPID for u64 {}
// impl Source for String {}

// fn delay(milis: u64) {
//     thread::sleep(Duration::from_millis(milis));
// }

// fn vote_req(v: u64, b: u64) -> Request<u64, String> {
//     return Request {
//         cmd: Command::Vote(v, Ballot::Player(b)),
//         src: format!("vote{}-{}", v, b).to_string(),
//     };
// }

// fn vote_abstain(v: u64) -> Request<u64, String> {
//     return Request {
//         cmd: Command::Vote(v, Ballot::Abstain),
//         src: format!("vote{}-abstain", v).to_string(),
//     };
// }

// fn vote_retract(v: u64) -> Request<u64, String> {
//     return Request {
//         cmd: Command::Vote(v, Ballot::Retract),
//         src: format!("vote{}-retract", v).to_string(),
//     };
// }

// fn action_req(a: u64, t: u64) -> Request<u64, String> {
//     return Request {
//         cmd: Command::Action(Actor::Player(a), Target::Player(t)),
//         src: format!("action{}-{}", a, t).to_string(),
//     };
// }

// fn mafia_action_req(m: u64, t: u64) -> Request<u64, String> {
//     return Request {
//         cmd: Command::Action(Actor::Mafia(m), Target::Player(t)),
//         src: format!("mafia{}-{}", m, t).to_string(),
//     };
// }

// fn send_and_recv(
//     req: Request<u64, String>,
//     tx: &Sender<Request<u64, String>>,
//     rx: &mut Receiver<Response<u64, String>>,
// ) {
//     tx.send(req).expect("Should send");
//     thread::sleep(Duration::from_millis(100));
//     empty_resp(rx);
// }

// fn recv_expected(
//     rx: &mut Receiver<Response<u64, String>>,
//     expected: Vec<Option<Response<u64, String>>>,
// ) {
//     // Allow wildcard?
//     for exp in expected {
//         let resp = rx.try_recv().expect("Should recv");
//         if let Some(exp) = exp {
//             assert_eq!(resp, exp);
//         }
//     }
// }

// #[derive(Debug)]
// pub enum CmpResp<'a> {
//     Wildcard,
//     SameEventType(&'a Event<u64>),
//     SameEvent(&'a Event<u64>),
//     SameEventTypeSameSrc(&'a Event<u64>, String),
//     SameEventSameSrc(&'a Event<u64>, String),
// }

// fn cmp_events<'a>(
//     found_vec: &Vec<Response<u64, String>>,
//     expected_vec: &Vec<CmpResp<'a>>,
// ) -> Result<(), String> {
//     let mut found_iter = found_vec.iter();
//     for expected in expected_vec {
//         let found = found_iter.next().ok_or("Not enough events found")?;

//         let fits = match &expected {
//             CmpResp::Wildcard => true,
//             CmpResp::SameEventType(event) => event.is_same_type(&found.event),
//             CmpResp::SameEvent(event) => event == &&found.event,
//             CmpResp::SameEventTypeSameSrc(event, src) => {
//                 event.is_same_type(&found.event) && src == &found.src
//             }
//             CmpResp::SameEventSameSrc(event, src) => event == &&found.event && src == &found.src,
//         };

//         if !fits {
//             return Err(format!("Expected {:?}, found {:?}", &expected, found).to_string());
//         }
//     }
//     Ok(())
// }

// fn get_std_events() -> HashMap<String, Event<u64>> {
//     let mut map = HashMap::new();
//     map.insert(
//         "Start".to_string(),
//         Event::Start {
//             players: Vec::new(),
//             phase: Phase::Init,
//         },
//     );
//     map.insert(
//         "Vote".to_string(),
//         Event::Vote {
//             voter: 0,
//             ballot: Ballot::Abstain,
//             former: None,
//             threshold: 0,
//             count: 0,
//         },
//     );
//     map.insert(
//         "Retract".to_string(),
//         Event::RetractVote {
//             voter: 0,
//             former: None,
//         },
//     );
//     map.insert(
//         "Election".to_string(),
//         Event::Election {
//             electors: Vec::new(),
//             ballot: Ballot::Abstain,
//         },
//     );
//     map.insert(
//         "Action".to_string(),
//         Event::Action {
//             actor: Actor::Player(0),
//             target: Target::Player(0),
//         },
//     );
//     map.insert(
//         "Strip".to_string(),
//         Event::Strip {
//             stripper: 0,
//             stripped: 0,
//         },
//     );
//     map.insert(
//         "Save".to_string(),
//         Event::Save {
//             doctor: 0,
//             saved: 0,
//         },
//     );
//     map.insert(
//         "Investigate".to_string(),
//         Event::Investigate {
//             cop: 0,
//             suspect: 0,
//             role: Role::TOWN,
//         },
//     );
//     map.insert(
//         "Kill".to_string(),
//         Event::Kill {
//             killer: 0,
//             victim: 0,
//         },
//     );
//     map.insert("Eliminate".to_string(), Event::Eliminate { player: 0 });

//     map.insert(
//         "End".to_string(),
//         Event::End {
//             winner: Winner::Team(Team::Town),
//         },
//     );

//     map
// }

// fn empty_resp(rx: &mut Receiver<Response<u64, String>>) -> Vec<Response<u64, String>> {
//     delay(100);
//     let mut resps = Vec::new();
//     loop {
//         if let Ok(resp) = rx.try_recv() {
//             resps.push(resp);
//             delay(500);
//         } else {
//             println!("-");
//             return resps;
//         }
//     }
// }

// fn try_join(game_thread: JoinHandle<Game<u64, String>>) -> Result<Game<u64, String>, ()> {
//     delay(200);

//     if game_thread.is_finished() {
//         return game_thread.join().map_err(|_| ());
//     } else {
//         return Err(());
//     }
// }

// fn get_std_players(n: usize) -> Vec<Player<u64>> {
//     let players = vec![
//         Player::new(1u64, "p1", Role::TOWN),
//         Player::new(2, "p2", Role::COP),
//         Player::new(3, "p3", Role::MAFIA),
//         Player::new(4, "p4", Role::DOCTOR),
//         Player::new(5, "p5", Role::CELEB),
//         Player::new(6, "p6", Role::MILLER),
//         Player::new(7, "p7", Role::GODFATHER),
//         Player::new(8, "p8", Role::IDIOT),
//         Player::new(9, "p9", Role::STRIPPER),
//         Player::new(10, "p10", Role::GOON),
//         Player::new(11, "p11", Role::SURVIVOR),
//         Player::new(13, "p13", Role::GUARD(1)), // p1
//         Player::new(14, "p14", Role::AGENT(2)), // p2
//         Player::new(12, "p12", Role::MASON),
//     ];

//     let players = Vec::from(&players[..n]);

//     return players;
// }

// fn get_std_comm() -> (
//     Sender<Request<u64, String>>,
//     Comm<u64, String>,
//     Receiver<Response<u64, String>>,
// ) {
//     let (tx, game_rx) = mpsc::channel();
//     let (game_tx, rx) = mpsc::channel();

//     let mut comm = Comm::new(game_rx, game_tx);

//     comm.save = SaveStrategy::PerChange("data/testgame.json".to_string());

//     return (tx, comm, rx);
// }

// fn std_game_setup(
//     players: Vec<Player<u64>>,
// ) -> (
//     Sender<Request<u64, String>>,
//     Receiver<Response<u64, String>>,
//     JoinHandle<Game<u64, String>>,
// ) {
//     let (tx, comm, rx) = get_std_comm();

//     let mut game = Game::new(players, comm);

//     let game_thread = match game.start() {
//         Ok(t) => t,
//         Err(e) => panic!("Error starting game: {:?}", e),
//     };
//     thread::sleep(Duration::from_millis(500));
//     return (tx, rx, game_thread);
// }

// #[test]
// #[ignore]
// fn run_basic() {
//     let players = vec![
//         Player::new(1u64, "p1", Role::TOWN),
//         Player::new(2, "p2", Role::TOWN),
//         Player::new(3, "p3", Role::MAFIA),
//     ];

//     let (tx, mut rx, game_thread) = std_game_setup(players);

//     tx.send(vote_req(1, 3)).unwrap();
//     tx.send(vote_req(2, 3)).unwrap();

//     empty_resp(&mut rx);

//     game_thread.join().unwrap();
// }

// #[test]
// #[ignore]
// fn run_night() {
//     let players = vec![
//         Player::new(1u64, "p1", Role::TOWN),
//         Player::new(2, "p2", Role::TOWN),
//         Player::new(3, "p3", Role::MAFIA),
//         Player::new(4, "p4", Role::TOWN),
//     ];

//     let (tx, mut rx, game_thread) = std_game_setup(players);

//     send_and_recv(mafia_action_req(3, 4), &tx, &mut rx);
//     send_and_recv(vote_req(1, 2), &tx, &mut rx);
//     send_and_recv(vote_req(2, 2), &tx, &mut rx);

//     try_join(game_thread).expect("Game should have ended");
// }

// #[test]
// fn add_player() {
//     let (tx, comm, mut rx) = get_std_comm();
//     let mut game = Game {
//         players: get_std_players(3),
//         comm,
//         phase: Phase::Init,
//     };

//     let p = Player::new(100, "p100", Role::TOWN);

//     game.add_player(p).expect("Should add just fine");

//     assert_eq!(
//         game.players,
//         vec![
//             Player::new(1u64, "p1", Role::TOWN),
//             Player::new(2, "p2", Role::COP),
//             Player::new(3, "p3", Role::MAFIA),
//             Player::new(100, "p100", Role::TOWN),
//         ]
//     );

//     let p = Player::new(100, "P-100", Role::TOWN);
//     game.add_player(p).expect_err("Player already exists");

//     assert_eq!(
//         game.players,
//         vec![
//             Player::new(1u64, "p1", Role::TOWN),
//             Player::new(2, "p2", Role::COP),
//             Player::new(3, "p3", Role::MAFIA),
//             Player::new(100, "p100", Role::TOWN),
//         ]
//     );

//     game.phase = Phase::new_day(1);

//     let p = Player::new(101, "p101", Role::TOWN);
//     game.add_player(p).expect_err("Can't add in init phase");
// }

// #[test]
// fn check_player() {
//     let (tx, comm, mut rx) = get_std_comm();
//     let mut game = Game {
//         players: get_std_players(3),
//         comm,
//         phase: Phase::Init,
//     };

//     game.check_player(&1).expect("Player exists");
//     game.check_player(&100).expect_err("Player doesn't exist");
// }

// #[test]
// fn start() -> Result<(), ()> {
//     let (tx, comm, mut rx) = get_std_comm();
//     let mut game = Game {
//         players: get_std_players(2),
//         comm,
//         phase: Phase::Init,
//     };

//     let mut game = game
//         .start()
//         .expect_err("Should be game because Not enough players");
//     delay(10);
//     game.add_player(Player::new(4, "p4", Role::TOWN))?;
//     let mut game = game
//         .start()
//         .expect_err("Should be game because Only Town roles");
//     delay(10);
//     game.add_player(Player::new(5, "p5", Role::MAFIA))?;
//     game.add_player(Player::new(6, "p6", Role::MAFIA))?;
//     game.add_player(Player::new(7, "p7", Role::MAFIA))?;
//     let mut game = game
//         .start()
//         .expect_err("Should be game because Equal num town maf roles");
//     delay(10);
//     game.add_player(Player::new(8, "p8", Role::IDIOT))?;
//     game.phase = Phase::End(Winner::Team(Team::Mafia));
//     let mut game = game
//         .start()
//         .expect_err("Should be game because Wrong phase");
//     delay(10);
//     game.phase = Phase::Init;
//     let game = game.start().expect("Should be join handle if game starts");
//     delay(500);
//     tx.send(Request {
//         cmd: Command::Halt,
//         src: String::default(),
//     })
//     .unwrap();
//     try_join(game).expect("Game should have been over");
//     Ok(())
// }

// /*
// Player::new(1u64, "p1", Role::TOWN),
// Player::new(2, "p2", Role::COP),
// Player::new(3, "p3", Role::MAFIA),
// Player::new(4, "p4", Role::DOCTOR),
// Player::new(5, "p5", Role::CELEB),
// Player::new(6, "p6", Role::MILLER),
// Player::new(7, "p7", Role::GODFATHER),
//  */
// #[test]
// fn handle_day() {
//     let (tx, comm, mut rx) = get_std_comm();
//     let mut game = Game {
//         players: get_std_players(7),
//         comm,
//         phase: Phase::Init,
//     };

//     game.phase = Phase::new_day(1);

//     tx.send(vote_req(1, 2)).unwrap();
//     delay(10);
//     game.handle_day();
//     assert_eq!(
//         game.phase,
//         Phase::Day {
//             day_no: 1,
//             votes: vec![(0, Ballot::Player(1))]
//         }
//     );

//     tx.send(vote_abstain(2)).unwrap();
//     game.handle_day();
//     assert_eq!(
//         game.phase,
//         Phase::Day {
//             day_no: 1,
//             votes: vec![(0, Ballot::Player(1)), (1, Ballot::Abstain)]
//         }
//     );

//     tx.send(vote_retract(1)).unwrap();
//     game.handle_day();
//     assert_eq!(
//         game.phase,
//         Phase::Day {
//             day_no: 1,
//             votes: vec![(1, Ballot::Abstain)]
//         }
//     );

//     empty_resp(&mut rx);

//     // Handle invalid votes
//     tx.send(vote_req(100, 2)).unwrap();
//     game.handle_day();

//     tx.send(vote_req(1, 100)).unwrap();
//     game.handle_day();

//     tx.send(action_req(2, 1)).unwrap();
//     game.handle_day();

//     let inv = Event::InvalidCommand("".to_string());

//     let found = empty_resp(&mut rx);
//     let expected = vec![
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//     ];

//     cmp_events(&found, &expected).expect("Votes should trigger invalid command");
// }

// /*
// Player::new(1u64, "p1", Role::TOWN),
// Player::new(2, "p2", Role::COP),
// Player::new(3, "p3", Role::MAFIA),
// Player::new(4, "p4", Role::DOCTOR),
// Player::new(5, "p5", Role::CELEB),
// Player::new(6, "p6", Role::MILLER),
// Player::new(7, "p7", Role::GODFATHER),
//  */
// #[test]
// fn handle_night() {
//     let (tx, comm, mut rx) = get_std_comm();
//     let mut game = Game {
//         players: get_std_players(7),
//         comm,
//         phase: Phase::Init,
//     };

//     game.phase = Phase::new_night(1);

//     tx.send(action_req(2, 7)).unwrap();
//     delay(10);
//     game.handle_night();
//     assert_eq!(
//         game.phase,
//         Phase::Night {
//             night_no: 1,
//             actions: vec![(Actor::Player(1), Target::Player(6))]
//         }
//     );

//     tx.send(action_req(4, 4)).unwrap();
//     game.handle_night();
//     assert_eq!(
//         game.phase,
//         Phase::Night {
//             night_no: 1,
//             actions: vec![
//                 (Actor::Player(1), Target::Player(6)),
//                 (Actor::Player(3), Target::Player(3))
//             ]
//         }
//     );

//     tx.send(mafia_action_req(7, 4)).unwrap();
//     game.handle_night();
//     assert_eq!(
//         game.phase,
//         Phase::Day {
//             day_no: 2,
//             votes: vec![]
//         }
//     );

//     empty_resp(&mut rx);

//     game.phase = Phase::new_night(2);

//     // Handle invalid actions
//     tx.send(action_req(1, 2)).unwrap();
//     game.handle_night();
//     tx.send(action_req(100, 2)).unwrap();
//     game.handle_night();
//     tx.send(action_req(2, 100)).unwrap();
//     game.handle_night();
//     tx.send(mafia_action_req(1, 2)).unwrap();
//     game.handle_night();
//     tx.send(mafia_action_req(3, 0)).unwrap();
//     game.handle_night();
//     tx.send(vote_req(2, 1)).unwrap();
//     game.handle_night();

//     let inv = Event::InvalidCommand("".to_string());

//     let found = empty_resp(&mut rx);
//     let expected = vec![
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//         CmpResp::SameEventType(&inv),
//     ];

//     cmp_events(&found, &expected).expect("Actions should trigger invalid command");
// }

// #[test]
// fn try_cmp_events() -> Result<(), ()> {
//     let init = Event::<u64>::Init;
//     let std_ev = get_std_events();
//     let mut expected = vec![
//         CmpResp::SameEvent(&Event::Init),
//         CmpResp::SameEventType(std_ev.get("Start").unwrap()),
//         CmpResp::SameEventTypeSameSrc(std_ev.get("Vote").unwrap(), "test".to_string()),
//     ];
//     let mut found: Vec<Response<u64, String>> = vec![
//         Response {
//             event: Event::Init,
//             src: String::default(),
//         },
//         Response {
//             event: Event::Start {
//                 players: vec![],
//                 phase: Phase::Init,
//             },
//             src: String::default(),
//         },
//         Response {
//             event: Event::Vote {
//                 voter: 1,
//                 ballot: Ballot::Player(2),
//                 former: None,
//                 count: 1,
//                 threshold: 2,
//             },
//             src: "test".to_string(),
//         },
//     ];

//     let result = cmp_events(&found, &expected);
//     assert!(result.is_ok());

//     found.push(Response {
//         event: Event::RetractVote {
//             voter: 1,
//             former: Some(Ballot::Player(2)),
//         },
//         src: "test".to_string(),
//     });

//     let result = cmp_events(&found, &expected);
//     assert!(result.is_ok());

//     expected.push(CmpResp::Wildcard);
//     expected.push(CmpResp::SameEventType(&Event::Night { night_no: 1 }));

//     assert!(result.is_ok());

//     found.push(Response {
//         event: Event::Vote {
//             voter: 2,
//             ballot: Ballot::Player(2),
//             former: None,
//             count: 1,
//             threshold: 2,
//         },
//         src: String::default(),
//     });

//     let result = cmp_events(&found, &expected);
//     assert!(result.is_err());

//     Ok(())
// }
