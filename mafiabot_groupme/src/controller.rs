use crate::prelude::*;

use groupme::Member;

mod parse;

pub enum ControllerMessge {
    CreateGame { members: Vec<Member>, rules: Rules, response: Resp<GameHandle> },
    DestroyGame { game_id: GameId },
    CreateLobby {},
    Message { data: Data },
}
pub struct Controller {
    rx: mpsc::Receiver<ControllerMessge>,
    lobbies: watch::Sender<HashMap<GroupId, LobbyHandle>>,
    games: watch::Sender<HashMap<GameId, GameHandle>>,
    focus: watch::Sender<HashMap<UserId, GameId>>,
}

impl Controller {
    async fn handle_msg(&mut self) {
        loop {
            use ControllerMessge::*;
            match self.rx.recv().await {
                Some(CreateGame { members, rules, response }) => {
                    todo!()
                }
                Some(DestroyGame { game_id }) => {
                    todo!()
                }
                Some(CreateLobby {}) => {
                    todo!()
                }
                Some(Message { data }) => {
                    todo!()
                }
                None => break,
            }
        }
    }
}

pub struct ControllerHandle {
    tx: mpsc::Sender<ControllerMessge>,
    lobbies: watch::Receiver<HashMap<GroupId, LobbyHandle>>,
    games: watch::Receiver<HashMap<GameId, GameHandle>>,
    focus: watch::Receiver<HashMap<UserId, GameId>>,
}

impl ControllerHandle {
    pub async fn start_game(&self, members: Vec<Member>, rules: Rules) -> GameHandle {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ControllerMessge::CreateGame { members, rules, response: tx })
            .await
            .expect("Controller should receive");
        rx.await.expect("Controller shouldn't drop tx")
    }
}
