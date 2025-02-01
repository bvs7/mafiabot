use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct GameHandle {
    pub game_id: GameId,
    pub main_chat_id: GroupId,
    pub mafia_chat_id: GroupId,
    action_tx: mpsc::Sender<(Action<W<UserId>>, Resp<Result<(), GameError>>)>,
    status: watch::Receiver<State>,
}

impl GameHandle {
    pub fn id(&self) -> GameId {
        self.game_id
    }
    pub async fn get_target(&self, target_ascii: u8) -> Result<Option<u64>, u8> {
        let state = self.status.borrow();
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
        let status = self.status.borrow().clone();
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
