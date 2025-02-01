use std::collections::HashSet;

use crate::prelude::*;

use groupme::Member;

mod parse;

pub enum ControllerMessge {
    CreateGame { lobby_id: GroupId, members: Vec<Member>, rules: Rules, response: Resp<GameHandle> },
    DestroyGame { game_id: GameId },
    CreateLobby { group_id: GroupId },
}

#[derive(Debug, Clone)]
pub struct ControllerHandle {
    tx: mpsc::Sender<ControllerMessge>,
    lobbies: watch::Receiver<HashMap<GroupId, LobbyHandle>>,
    games: watch::Receiver<HashMap<GameId, GameHandle>>,
    focus: watch::Receiver<HashMap<UserId, GameId>>,
}

impl ControllerHandle {
    pub async fn start_game(
        &self,
        lobby_id: GroupId,
        members: Vec<Member>,
        rules: Rules,
    ) -> GameHandle {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ControllerMessge::CreateGame { lobby_id, members, rules, response: tx })
            .await
            .expect("Controller should receive");
        rx.await.expect("Controller shouldn't drop tx")
    }
}
pub struct Controller {
    handle: ControllerHandle,
    lobbies: watch::Sender<HashMap<GroupId, LobbyHandle>>,
    games: watch::Sender<HashMap<GameId, GameHandle>>,
    focus: watch::Sender<HashMap<UserId, GameId>>,
    admins: HashSet<UserId>,
}

// TODO: set up subscriber on create!

impl Controller {
    pub async fn create(
        lobby_ids: Vec<GroupId>,
        admins: HashSet<UserId>,
    ) -> (JoinHandle<()>, ControllerHandle) {
        let (message_tx, message_rx) = mpsc::channel(32);
        let (lobbies, lobbies_rx) = watch::channel(HashMap::new());
        let (games, games_rx) = watch::channel(HashMap::new());
        let (focus, focus_rx) = watch::channel(HashMap::new());
        let handle = ControllerHandle {
            tx: message_tx,
            lobbies: lobbies_rx,
            games: games_rx,
            focus: focus_rx,
        };
        let controller = Self { handle: handle.clone(), lobbies, games, focus, admins };
        let task = tokio::spawn(controller.run(message_rx));
        for group_id in lobby_ids {
            handle
                .tx
                .send(ControllerMessge::CreateLobby { group_id })
                .await
                .expect("Controller should receive");
        }
        (task, handle)
    }

    async fn run(mut self, mut rx: mpsc::Receiver<ControllerMessge>) {
        loop {
            match rx.recv().await {
                Some(msg) => self.handle_msg(msg).await,
                None => break,
            }
        }
    }

    async fn handle_msg(&mut self, msg: ControllerMessge) {
        use ControllerMessge::*;
        match msg {
            CreateGame { lobby_id, members, rules, response } => {
                let game = create_game(lobby_id, members, rules).await;
                // Get targeters and change focuses
                let state = game.get_status().await;
                let players = state.players().alive();
                let mut focus = HashMap::new();
                for (pid, role) in players {
                    if role.is_targeting() {
                        focus.insert(W(pid).into(), game.id());
                    }
                }
                self.focus.send_modify(|focuses| {
                    focuses.extend(focus);
                });
                self.games.send_modify(|games| {
                    games.insert(game.id(), game.clone());
                });
                response.send(game).unwrap();
            }
            DestroyGame { game_id } => {
                if let Some(game) = self.games.borrow().get(&game_id) {
                    game.stop();
                    self.games.send_modify(|games| {
                        games.remove(&game_id);
                    });
                }
            }
            CreateLobby { group_id } => {
                info!("Creating lobby for group {}", group_id);
                let lobby = Lobby::create(group_id, self.handle.clone()).await;
                self.lobbies.send_modify(|lobbies| {
                    lobbies.insert(group_id, lobby);
                });
            }
        }
    }
}
