use crate::prelude::*;

use crate::game_handler::action_handler::ActionTx;

/*
Thoughts on Full App State
Shared app state seems good, as it allows for easy read access. The only time it needs to be written
is to add or remove games, groups, or lobbies.

Seems like starting the handlers is a little messy right now.

Status can't get names until the game is started and players are added...
*/

#[derive(Debug)]
pub struct GameInfo {
    pub game_id: GameId,
    pub main_id: GroupId,
    pub mafia_id: GroupId,
    pub status: watch::Receiver<Status>,
    pub action_tx: ActionTx,
}

impl GameInfo {
    pub fn new(
        game_id: GameId,
        main_id: GroupId,
        mafia_id: GroupId,
        status: watch::Receiver<Status>,
        action_tx: ActionTx,
    ) -> Self {
        Self { game_id, main_id, mafia_id, status, action_tx }
    }
}

#[derive(Debug)]
pub struct LobbyInfo {
    pub lobby_id: GroupId,
    pub chat: GroupMeGroup,
}

#[derive(Debug)]
pub struct AppStatus {
    pub games: HashMap<GameId, GameInfo>,
    pub groups: HashMap<GroupId, GroupMeGroup>,
    pub lobbies: HashMap<GroupId, LobbyInfo>,
}

impl AppStatus {
    pub fn new() -> Self {
        Self { games: HashMap::new(), groups: HashMap::new(), lobbies: HashMap::new() }
    }
}
