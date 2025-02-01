use crate::prelude::*;

mod event_handler;
use event_handler::EventHandler;

#[derive(Debug, Clone)]
pub struct GameHandle {
    pub game_id: GameId,
    pub main_chat_id: GroupId,
    pub mafia_chat_id: GroupId,
    action_tx: mpsc::Sender<(Action<W<UserId>>, Resp<Result<(), GameError>>)>,
    status_rx: watch::Receiver<State>,
    game_abort: AbortHandle,
    event_abort: AbortHandle,
}

pub async fn create_game(
    lobby_id: GroupId,
    mut members: Vec<groupme::Member>,
    rules: Rules,
) -> GameHandle {
    let players = members.iter().map(|m| W(m.user_id)).collect::<Vec<_>>();
    let game = mafia::game::Game::new(players, rules);
    let game_id = game.id();

    let mut main_chat = groupme::Group::new(format!("Game {}", game_id)).await;
    let main_chat_id = main_chat.id();
    let mut mafia_chat = groupme::Group::new(format!("Mafia {}", game_id)).await;
    let mafia_chat_id = mafia_chat.id();

    // Add members
    main_chat.add_members(members.clone()).await;

    let players = game.players();
    members.retain(|m| players.get(&W(m.user_id).into()).is_some_and(|p| p.is_mafia()));
    mafia_chat.add_members(members).await;

    let event_handler = EventHandler::new(game_id, lobby_id, main_chat, mafia_chat, players);

    let (game_task, action_tx, status_rx, event_rx) = game.start::<W<UserId>>();
    let event_task = tokio::spawn(event_handler.run(event_rx));

    let game_abort = game_task.abort_handle();
    let event_abort = event_task.abort_handle();

    GameHandle {
        game_id,
        main_chat_id,
        mafia_chat_id,
        action_tx,
        status_rx,
        game_abort,
        event_abort,
    }
}

impl GameHandle {
    pub fn stop(&self) {
        self.game_abort.abort();
        self.event_abort.abort();
    }

    pub fn id(&self) -> GameId {
        self.game_id
    }
    pub async fn get_target(&self, target_ascii: u8) -> Result<Option<u64>, u8> {
        let state = self.status_rx.borrow();
        let players = state.players().list();
        let target_ascii = target_ascii.to_ascii_uppercase();

        let idx = target_ascii.checked_sub(b'A').map(usize::from);
        match idx {
            Some(idx) if idx > players.len() => Err(target_ascii),
            Some(idx) if idx == players.len() => Ok(None),
            Some(idx) => Ok(Some(players[idx as usize].0)),
            None => Err(target_ascii),
        }
    }
    pub async fn send_action(&self, action: Action<W<UserId>>) -> Result<(), GameError> {
        let (tx, rx) = oneshot::channel();
        self.action_tx.send((action, tx)).await.expect("Game should receive");
        rx.await.expect("Game shouldn't drop tx")
    }
    pub async fn get_status(&self) -> State {
        let status = self.status_rx.borrow().clone();
        status
    }
    pub async fn get_brief(&self) -> Brief {
        let state = self.get_status().await;
        state.into()
    }

    pub async fn perform_game_cmd(&self, cmd: GameCommand, resp: RespContext) {
        use GameCommand::*;
        let action_result = match cmd {
            Vote { user_id: voter, ballot } => {
                let action = Action::Vote { voter, ballot };
                self.send_action(action).await
            }
            Reveal { user_id: actor } => {
                let action = Action::Reveal { actor };
                self.send_action(action).await
            }
            Target { user_id: actor, target: choice } => {
                let action = Action::Target { actor, choice };
                self.send_action(action).await
            }
            Scheme { user_id: killer, target: mark } => {
                let action = Action::Scheme { killer, mark };
                self.send_action(action).await
            }
            Status => {
                let status = self.get_status().await;
                let msg = format!("Game {}: {}", self.game_id, status);
                let _ = api::send_group_message(&self.main_chat_id, &msg).await;
                Ok(())
            }
        };
        match action_result {
            Ok(()) => match resp {
                RespContext::Group(group_id, msg_id) => {
                    let _ = api::like_group_message(&group_id, &msg_id).await;
                }
                RespContext::User(user_id, msg_id) => {
                    let _ = api::like_dm_message(&user_id, &msg_id).await;
                }
            },
            Err(e) => match resp {
                RespContext::Group(group_id, msg_id) => {
                    let msg = format!("Action Failed: {}", e);
                    let _ = api::send_group_message(&group_id, &msg).await;
                }
                RespContext::User(user_id, msg_id) => {
                    let msg = format!("Action Failed: {}", e);
                    let _ = api::send_dm(user_id, &msg).await;
                }
            },
        }
    }
}
