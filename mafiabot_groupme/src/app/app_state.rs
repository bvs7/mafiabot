use chrono::{DateTime, Local};
use std::{collections::HashSet, time::Duration};
use tokio::{sync::Mutex, task::AbortHandle};

use crate::{
    app::{self, app_state},
    prelude::*,
};

#[derive(Debug)]
pub struct Lobby {
    group_id: GroupId,
    start_msg: Arc<Mutex<Option<(MessageId, AbortHandle)>>>,
    update: tokio::sync::Notify,
    games: HashSet<GameId>,
    rules: Rules,
    app_state: Arc<AppState>,
}

impl Lobby {
    pub async fn parse_cmd(
        &self,
        user_id: UserId,
        words: Vec<String>,
        attachments: Vec<Attachment>,
        app_state: &Arc<AppState>,
    ) -> bool {
        // Check for... start command
        let Some(first) = words.first() else {
            return false;
        };
        if first == "/start" {
            let mut minutes = 5;
            if let Some(Ok(min)) = words.get(1).map(|s| s.parse::<u64>()) {
                minutes = min;
                if minutes > 120 {
                    minutes = 120;
                } else if minutes < 1 {
                    minutes = 1;
                }
            }
            let mut min_players = 5;
            if let Some(Ok(min)) = words.get(2).map(|s| s.parse::<usize>()) {
                min_players = min;
                if min_players < 3 {
                    min_players = 3;
                }
            }
            if let Some((msg_id, abort_handle)) = self.start_msg.lock().await.take() {
                abort_handle.abort();
            }
            let msg_id = api::send_group_message(
                &self.group_id,
                &format!(
                    "Game starting in {minutes} minutes, if {min_players} players join. Like \
                this message to join"
                ),
            )
            .await
            .unwrap();
            let end_time = Duration::from_secs(minutes * 60);
            let abort_handle = tokio::spawn({
                let lobby_id = self.group_id.clone();
                let msg_id = msg_id.clone();
                let rules = self.rules.clone();
                let app_state = self.app_state.clone();
                async move {
                    tokio::time::sleep(Duration::from_secs(minutes * 60)).await;
                    let message_resp = api::get_group_message(&lobby_id, &msg_id).await.unwrap();
                    let user_ids = message_resp.favorited_by;
                    let names = app_state.get_names(&lobby_id).await.clone();
                    let members = user_ids
                        .into_iter()
                        .map(|id| groupme::Member::new(names.get(&id).unwrap().to_string(), id))
                        .collect();
                    let game_id = app_state.create_game(members, rules, lobby_id.clone()).await;
                    let mut w_lobbies = app_state.lobbies.write().await;
                    let lobby = w_lobbies.get_mut(&lobby_id).unwrap();
                    lobby.games.insert(game_id);
                }
            })
            .abort_handle();
            let mut start_msg = self.start_msg.lock().await;
            *start_msg = Some((msg_id, abort_handle));
            drop(start_msg);
            self.update.notify_one();
        }
        todo!()
    }
}

#[derive(Debug)]
pub struct AppState {
    pub lobbies: RwLock<HashMap<GroupId, Lobby>>,
    pub games: RwLock<HashMap<GameId, GameHandler>>,
    pub groups: RwLock<HashMap<GroupId, groupme::Group>>,
    pub focus: RwLock<HashMap<UserId, GameId>>,
    // pub api_tx: mpsc::Sender<()>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            lobbies: RwLock::new(HashMap::new()),
            games: RwLock::new(HashMap::new()),
            groups: RwLock::new(HashMap::new()),
            focus: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_group(self: &Arc<Self>, name: String) -> GroupId {
        let group = groupme::Group::new(name).await;
        let id = group.id().clone();
        let mut w_groups = self.groups.write().await;
        w_groups.insert(id.clone(), group);
        drop(w_groups);
        id
    }

    pub async fn create_game(
        self: &Arc<Self>,
        members: Vec<groupme::Member>,
        rules: Rules,
        lobby_id: GroupId,
    ) -> GameId {
        let game = GameHandler::new(self.clone(), members.clone(), rules, lobby_id).await;
        let id = game.id();
        let mut w_games = self.games.write().await;
        w_games.insert(id, game);
        drop(w_games);
        let users = members.iter().map(|m| m.user_id.clone()).collect::<Vec<_>>();
        for user_id in users {
            let mut w_focus = self.focus.write().await;
            w_focus.insert(user_id, id);
        }
        id
    }

    pub async fn get_name(
        self: &Arc<Self>,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Option<String> {
        for _ in 0..3 {
            let r_groups = self.groups.read().await;
            let group = r_groups.get(group_id).unwrap();
            let name = group.name(user_id).clone();
            match name {
                Some(name) => return Some(name.to_owned()),
                None => {
                    self.update_names(group_id).await;
                }
            }
        }
        error!("Failed to get name for user_id: {}", user_id);
        None
    }

    pub async fn get_names(self: &Arc<Self>, group_id: &GroupId) -> HashMap<UserId, String> {
        let r_groups = self.groups.read().await;
        let group = r_groups.get(group_id).unwrap();
        group.names()
    }

    pub async fn update_names(self: &Arc<Self>, group_id: &GroupId) {
        let mut w_groups = self.groups.write().await;
        let group = w_groups.get_mut(group_id).unwrap();
        group.update_names().await;
    }
}
