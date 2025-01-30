use crate::prelude::*;

use tokio::task::JoinSet;
use tokio_stream::wrappers::ReceiverStream;

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
    }
}
