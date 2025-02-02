use std::time::Duration;

use groupme::Member;
use tokio::time::{error::Elapsed, Instant};

use crate::prelude::*;

pub struct Lobby {
    lobby_chat: groupme::Group,
    games_tx: watch::Sender<HashMap<GameId, GameHandle>>,
    start_msg_rx: watch::Receiver<Option<(MessageId, usize, Instant)>>,
    start_msg_tx: watch::Sender<Option<(MessageId, usize, Instant)>>,
    rules: Rules,
    ctrl_handle: ControllerHandle,
}

impl Lobby {
    pub async fn create(lobby_chat_id: GroupId, ctrl_handle: ControllerHandle) -> LobbyHandle {
        let (start_msg_tx, start_msg_rx) = watch::channel(None);
        let (games_tx, games_rx) = watch::channel(HashMap::new());
        let rules = Rules::default();
        let lobby_chat = groupme::Group::from_id(lobby_chat_id).await;
        let lobby = Self {
            lobby_chat,
            games_tx,
            start_msg_rx,
            start_msg_tx: start_msg_tx.clone(),
            rules,
            ctrl_handle,
        };
        let handle = tokio::spawn(lobby.run());
        let lobby_abort = handle.abort_handle();
        LobbyHandle { lobby_chat_id, games_rx, start_msg_tx, lobby_abort }
    }

    async fn run(mut self) -> JoinHandle<()> {
        let mut start_msg;
        loop {
            start_msg = self.start_msg_rx.borrow_and_update().clone();
            if let Some((msg_id, min_players, start_time)) = start_msg {
                match tokio::time::timeout_at(start_time, self.start_msg_rx.changed()).await {
                    Err(elapsed) => {
                        // Unset Start time
                        self.start_msg_tx.send(None).unwrap();
                        self.start_game(msg_id, min_players).await;
                    }
                    Ok(_) => continue,
                }
            }
            tokio::task::yield_now().await;
        }
    }

    async fn start_game(&mut self, msg_id: MessageId, min_players: usize) {
        let lobby_id = self.lobby_chat.id();
        let msg = api::get_group_message(&lobby_id, &msg_id).await.unwrap();
        let users = msg.favorited_by;
        if users.len() < min_players {
            let _ = api::send_group_message(&lobby_id, "Not enough players to start game").await;
            return;
        }
        self.lobby_chat.update_names().await;
        let members = users
            .into_iter()
            .map(|u_id| {
                let name = self.lobby_chat.name(&u_id).unwrap_or_else(|| format!("User {}", u_id));
                Member::new(name, u_id)
            })
            .collect();
        let game_handle = self.ctrl_handle.start_game(lobby_id, members, self.rules.clone()).await;
        self.games_tx.send_modify(|games| {
            games.insert(game_handle.id(), game_handle);
        });
    }
}

#[derive(Debug, Clone)]
pub struct LobbyHandle {
    lobby_chat_id: GroupId,
    games_rx: watch::Receiver<HashMap<GameId, GameHandle>>, // TODO should this be internal? A watch?
    start_msg_tx: watch::Sender<Option<(MessageId, usize, Instant)>>,
    lobby_abort: AbortHandle,
}

impl LobbyHandle {
    pub async fn send_start_msg(&self, minutes: u64, min_players: usize) {
        let msg = format!(
            "Starting a game in {} minutes if at least {} players join. Like this message to join!",
            minutes, min_players
        );
        let msg_id =
            api::send_group_message(&self.lobby_chat_id, &msg).await.expect("Message should send");
        let start_time = Instant::now() + Duration::from_secs(minutes * 60);
        self.start_msg_tx
            .send(Some((msg_id, min_players, start_time)))
            .expect("Lobby should receive");
    }

    pub async fn perform_lobby_cmd(&self, cmd: LobbyCommand, resp: RespContext) {
        use LobbyCommand::*;
        match cmd {
            Start { minutes, min_players } => self.send_start_msg(minutes, min_players).await,
            Status => {
                let mut msg = String::new();
                let games = self.games_rx.borrow().clone();
                if games.is_empty() {
                    msg.push_str("No games in lobby");
                } else {
                    msg.push_str("Games in lobby:");
                    for game in games.values() {
                        let brief = game.get_brief().await;
                        msg.push_str(&format!("\n{}", brief));
                    }
                }
                let _ = api::send_group_message(&self.lobby_chat_id, &msg).await;
            }
            StatusOf { game_id } => {
                let games = self.games_rx.borrow().clone();
                let Some(game) = games.get(&game_id) else {
                    let _ = api::send_group_message(&self.lobby_chat_id, "Game not found").await;
                    return;
                };
                let brief = game.get_brief().await;
                let msg = format!("{}", brief);
                let _ = api::send_group_message(&self.lobby_chat_id, &msg).await;
            }
        }
    }
}
