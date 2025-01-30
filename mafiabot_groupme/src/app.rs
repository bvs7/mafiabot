use broadcast::error::RecvError;
use rand::thread_rng;

use crate::prelude::*;

<<<<<<< HEAD
use tokio::task::JoinSet;
use tokio_stream::wrappers::ReceiverStream;
=======
use crate::game_handler::action_handler::ActionTx;
use groupme::subscriber::{Data, PushWebSocketServer};
>>>>>>> 08e3739c045b00dd4227b8176a57a1d7d2b3a843

// If splitting locks... never wait on a higher lock.

#[derive(Debug)]
pub struct AppState {
    pub lobbies: RwLock<HashMap<GroupId, Lobby>>,
    pub games: RwLock<HashMap<GameId, GameHandler>>,
    pub groups: RwLock<HashMap<GroupId, groupme::Group>>,
    // pub api_tx: mpsc::Sender<()>,
}

impl AppState {
    pub async fn create_group(self: &Arc<Self>, name: String) -> GroupId {
        let mut group = groupme::Group::new(name).await;
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

#[derive(Debug, Clone)]
pub struct Lobby {}

enum Error {}

enum Cmd {}
enum Resp {}

/// Return type from running all of the futures in the task list. When one of these returns
enum TaskResult {
    Game(Game),
    Lobby(Lobby),
}

async fn game() -> Result<TaskResult, Error> {
    todo!()
}

async fn lobby() -> Result<TaskResult, Error> {
    todo!()
}

struct App {
    js: JoinSet<TaskResult>,
    app_req_rx: mpsc::Receiver<AppRequest>, // TODO: how does the tx get sent to handlers?
    cmd: ReceiverStream<(Cmd, Resp)>,       // TODO: Implement
}
impl App {
    async fn run(&mut self) {
        loop {
            tokio::select! {
                // Check if a new app level request has been received
                app_req = self.app_req_rx.recv() => {
                    let Some(app_req) = app_req else {
                        // If None is returned, the app_req channel has been closed...
                        // Seems like this can't happen if we hold a tx to the channel
                        break;
                    };
                    self.handle_app_request(app_req).await;
                }
                // Check if a new command has been received
                // c = self.cmd.next() => {
                //     todo!()
                // }
                // If None is returned, no tasks will be added until rx.recv() returns one
                Some(tr) = self.js.join_next() => {
                    // TODO: Handle join error?
                    self.handle_task_result(tr.unwrap()).await;
                }
            }
        }
    }

    async fn handle_task_result(&mut self, task_result: TaskResult) {
        match task_result {
            TaskResult::Game(game) => {
                todo!()
            }
            TaskResult::Lobby(lobby) => {
                todo!()
            }
        }
    }
}

enum AppRequest {
    CreateGame { users: Vec<UserId>, rules: Rules, resp: oneshot::Sender<Result<GameId, Error>> },
    CreateGroup { members: Vec<(UserId, String)>, resp: oneshot::Sender<Result<GroupId, Error>> },
}

impl App {
    async fn handle_app_request(&mut self, req: AppRequest) {
        todo!()
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
