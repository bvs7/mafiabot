use std::fmt::format;

use crate::{app::GameInfo, prelude::*};
use mafia::game::ActionHandler;

use async_trait::async_trait;

// How to create a new action handler

pub struct GroupMeActionHandler {
    game_id: GameId,
    main_id: GroupId,
    mafia_id: GroupId,
    action_rx: mpsc::Receiver<(Action<u64>, oneshot::Sender<Result<(), Error>>)>,
    last_resp: Option<oneshot::Sender<Result<(), Error>>>,
    status_tx: watch::Sender<Status>,
    app_status: Arc<RwLock<AppStatus>>,
}

pub type ActionTx = mpsc::Sender<(Action<u64>, oneshot::Sender<Result<(), Error>>)>;

impl GroupMeActionHandler {
    pub async fn new(game: &Game, app_status: Arc<RwLock<AppStatus>>) -> Self {
        let (action_tx, action_rx) = mpsc::channel(1);
        let (status_tx, status_rx) = watch::channel(Status::default());

        // Create Groups
        let main_id = GroupMeGroup::new(format!("Main Chat #{}", game.id()), &app_status).await;
        let mafia_id = GroupMeGroup::new(format!("Mafia Chat #{}", game.id()), &app_status).await;

        let game_id = game.id();
        let game_info = GameInfo::new(
            game_id,
            main_id.clone(),
            mafia_id.clone(),
            status_rx.clone(),
            action_tx.clone(),
        );

        let mut w_app_status = app_status.write().await;
        w_app_status.games.insert(game_id, game_info);
        drop(w_app_status);

        let handler =
            Self { game_id, main_id, mafia_id, action_rx, last_resp: None, status_tx, app_status };
        handler
    }
}
#[async_trait]
impl ActionHandler for GroupMeActionHandler {
    type PID = u64;
    async fn recv_action(&mut self) -> Option<Action<u64>> {
        let (action, resp) = match self.action_rx.recv().await {
            Some(payload) => payload,
            None => return None,
        };
        self.last_resp = Some(resp);
        Some(action)
    }
    async fn resp_action(&mut self, result: Result<(), Error>) {
        if let Some(resp) = self.last_resp.take() {
            let _ = resp.send(result);
        }
    }
    async fn update_status(&mut self, state: &State) {
        let r_app_status = self.app_status.read().await;
        let names =
            r_app_status.groups.get(&self.main_id).expect("Main chat not found").names.clone();
        let status = state.status(&names);
        let _ = self.status_tx.send(status);
    }
}
