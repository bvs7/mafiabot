use crate::prelude::*;

#[derive(Debug)]
pub struct AppState {
    // pub lobbies: RwLock<HashMap<GroupId, Lobby>>,
    pub games: RwLock<HashMap<GameId, GameHandler>>,
    pub groups: RwLock<HashMap<GroupId, groupme::Group>>,
    // pub api_tx: mpsc::Sender<()>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            // lobbies: RwLock::new(HashMap::new()),
            games: RwLock::new(HashMap::new()),
            groups: RwLock::new(HashMap::new()),
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
    ) -> GameId {
        let game = GameHandler::new(self.clone(), members, rules).await;
        let id = game.id();
        let mut w_games = self.games.write().await;
        w_games.insert(id, game);
        drop(w_games);
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
