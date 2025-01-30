use broadcast::error::RecvError;
use rand::thread_rng;

use crate::prelude::*;

use crate::game_handler::action_handler::ActionTx;
use groupme::subscriber::{Data, PushWebSocketServer};

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
    pub rules: Rules,
}

#[derive(Debug, Default)]
pub struct AppStatus {
    pub games: HashMap<GameId, GameInfo>,
    pub groups: HashMap<GroupId, GroupMeGroup>,
    pub lobbies: HashMap<GroupId, LobbyInfo>,
    pub focuses: HashMap<UserId, GameId>,
}

impl AppStatus {
    pub fn new() -> Self {
        Self::default()
    }
}

mod parse {
    use broadcast::error::RecvError;
    use groupme::subscriber::{Data, PushWebSocketServer};

    use crate::{
        app::parse,
        game_handler::{action_handler, event_handler},
        prelude::*,
    };

    #[derive(Debug, Clone)]
    pub enum Cmd {
        Lobby(GroupId, UserId, LobbyCmd),
        Game(GameId, UserId, GameCmd),
    }

    #[derive(Debug, Clone)]
    pub enum LobbyCmd {
        Start(usize, usize),
        Status(Option<GameId>),
    }

    // TODO: don't let this be raw
    #[derive(Debug, Clone)]
    pub enum GameCmd {
        Vote(Option<Option<UserId>>),
        Reveal,
        Target(Option<UserId>),
        Scheme(Option<UserId>),
        Status,
    }

    #[derive(Debug, Clone)]
    pub enum Response {
        Group(MessageId, GroupId),
        DM(MessageId, UserId),
    }

    async fn create_game(lobby_id: &GroupId, ids: Vec<u64>, app_status: &Arc<RwLock<AppStatus>>) {
        let r_app_status = app_status.read().await;
        let lobby = r_app_status.lobbies.get(lobby_id).unwrap();
        // Rolegen
        let rules = lobby.rules.clone();
        drop(r_app_status);
        let mut rolegen = mafia::rolegen::DrawRoleGen::new(mafia::rules::Rules::default());
        let registry = rolegen.generate_roles(ids, &rules);
        let game = Game::new(registry, rules);
        let action_handler = GroupMeActionHandler::new(&game, app_status.clone()).await;
        let event_handler =
            GroupMeEventHandler::new(&game, lobby_id.clone(), app_status.clone()).await;
        // Just start?
        let id = game.id();
        tokio::spawn(game.run(action_handler, event_handler));
    }

    async fn message_handler(app_status: Arc<RwLock<AppStatus>>) {
        // Create subscriber
        let mut server = PushWebSocketServer::new();
        let mut rx = server.get_rx();

        server.start().unwrap();

        loop {
            match rx.recv().await {
                Ok(data) => {}
                Err(RecvError::Lagged(n)) => {
                    warn!("Lagged: {n}");
                }
                Err(RecvError::Closed) => {
                    break;
                }
            }
        }
    }

    async fn parse_cmd(cmd: Data, app_status: Arc<RwLock<AppStatus>>) -> Option<(Cmd, Response)> {
        match cmd {
            Data::GroupMsg { attachments, group_id, created_at, id, name, text, user_id } => {
                // TODO: Update name

                let c1 = text.chars().next()?;
                if c1 != '/' {
                    return None;
                }
                parse_group_cmd(group_id, user_id, text, attachments, app_status).await
            }
            Data::DirectMsg { attachments, created_at, id, name, text, user_id } => {
                let c1 = text.chars().next()?;
                if c1 != '/' {
                    return None;
                }
                parse_dm_cmd(user_id, text, app_status).await
            }
            Data::Unknown => None,
        }
    }

    async fn parse_group_cmd(
        group_id: GroupId,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
        app_status: Arc<RwLock<AppStatus>>,
    ) -> Option<(Cmd, Response)> {
        let r_app_status = app_status.read().await;
        if let Some(lobby) = r_app_status.lobbies.get(&group_id) {
            drop(r_app_status);
            return parse_lobby_cmd(group_id, user_id, text, attachments, app_status).await;
        }
        // Not a lobby, check if it's a game
        else {
            for game in r_app_status.games.values() {
                if game.main_id == group_id {
                    let id = game.game_id;
                    drop(r_app_status);
                    return parse_main_chat_cmd(id, user_id, text, attachments, app_status).await;
                } else if game.mafia_id == group_id {
                    let id = game.game_id;
                    drop(r_app_status);
                    return parse_mafia_chat_cmd(id, user_id, text, attachments, app_status).await;
                }
            }
        }
        return None;
    }

    async fn parse_lobby_cmd(
        lobby_id: GroupId,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
        app_status: Arc<RwLock<AppStatus>>,
    ) -> Option<(Cmd, Response)> {
        let r_app_status = app_status.read().await;
        let lobby = r_app_status.lobbies.get(&lobby_id)?;
        drop(r_app_status);

        let words = text.split_whitespace().collect::<Vec<&str>>();
        if words.len() == 0 {
            return None;
        }
        let cmd = words[0];
        if cmd == "/admin_start" {
            for a in attachments {
                if let Attachment::Mentions { user_ids } = a {
                    create_game(&lobby_id, user_ids, &app_status).await;
                }
            }
            // TODO: unwrap
        }
        return None;
    }

    async fn parse_main_chat_cmd(
        game_id: GameId,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
        app_status: Arc<RwLock<AppStatus>>,
    ) -> Option<(Cmd, Response)> {
        let words = text.split_whitespace().collect::<Vec<&str>>();
        if words.len() == 0 {
            return None;
        }
        let cmd = words[0];
        if cmd == "/admin_start" {}
        return None;
    }

    async fn parse_mafia_chat_cmd(
        game_id: GameId,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
        app_status: Arc<RwLock<AppStatus>>,
    ) -> Option<(Cmd, Response)> {
        todo!()
    }

    async fn parse_dm_cmd(
        user_id: UserId,
        text: String,
        app_status: Arc<RwLock<AppStatus>>,
    ) -> Option<(Cmd, Response)> {
        todo!()
    }
}
/*
TODO:
- admin_start command...
- create_game function
- subscriber handler
*/
