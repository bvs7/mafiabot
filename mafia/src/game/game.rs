// use std::{env, future::Future, path::PathBuf, sync::Arc};

// use tracing::event;

// use crate::{prelude::*, state};

// use super::action;

// // TODO: What if there is a resp type for each action?
// // Instead of having so many events, just have a response type for each action.
// // For example, vote response holds ballot, former, counts, etc.
// // Reveal response would need to be an event...
// // scheme, target, eclipse_vote could just be responses as well?
// pub trait ActionResp {
//     fn send(self, result: Result<(), Error>);
// }

// // TODO: Change trait name to include updating
// pub trait ActionQueue {
//     type PID: Into<Pid> + Copy;
//     type Resp: ActionResp;
//     fn recv(&mut self) -> Option<(Action<Self::PID>, Self::Resp)>;
//     fn update(&mut self, status: Status);
// }

// /*
// Game has an action handler?, state has an event handler.

// */
// #[derive(Debug, Clone)]
// pub struct Game<A, E> {
//     pub id: GameId,
//     action_queue: A,
//     state: Arc<RwLock<State<E>>>,
// }

// impl<A, E> Game<A, E> {
//     pub fn new(
//         players: impl IntoIterator<Item = impl Into<Pid>>,
//         rules: Rules,
//         action_queue: A,
//         event_handler: E,
//     ) -> Self
// where {
//         let id = GameId::new().unwrap_or_default();
//         Self::with_id(id, players, rules, action_queue, event_handler)
//     }

//     pub fn with_id(
//         id: GameId,
//         players: impl IntoIterator<Item = impl Into<Pid>>,
//         rules: Rules,
//         action_queue: A,
//         event_handler: E,
//     ) -> Self {
//         let state = State::new(players, rules, event_handler);
//         let state = Arc::new(RwLock::new(state));
//         let id = GameId::new().unwrap_or_default();
//         Self { id: GameId::new().unwrap_or_default(), action_queue, state }
//     }

//     // Blocking fn to run the game
//     pub fn run(mut self) -> Self
//     where
//         A: ActionQueue,
//         E: EventSender,
//     {
//         let mut w_state = self.state.write().unwrap();
//         // If the game is not started (in phase Init), start it?
//         let status = w_state.status();
//         if status.phase == PhaseKind::Init {
//             w_state.start();
//         }
//         w_state.update();
//         self.action_queue.update(w_state.status());
//         drop(w_state);

//         loop {
//             let Some((action, resp)) = self.action_queue.recv() else {
//                 // TODO: handle action queue closed, save game?
//                 break;
//             };
//             let r_state = self.state.read().unwrap();
//             let result = r_state.validate_action(action).map(|_| ());
//             drop(r_state);
//             if result.is_err() {
//                 resp.send(result);
//                 continue;
//             }
//             let mut w_state = self.state.write().unwrap();
//             match w_state.validate_action(action) {
//                 Ok(valid) => {
//                     w_state.perform_action(valid);
//                     resp.send(Ok(()));
//                 }
//                 Err(err) => {
//                     resp.send(Err(err));
//                     continue;
//                 }
//             }

//             w_state.update();
//             drop(w_state);
//         }
//         self
//     }
// }
