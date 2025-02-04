use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::DateTime;
use mafia::{
    game::{self, GameId},
    state::players,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::prelude::*;

type GameResp = Resp<Result<(), GameError>>;
type GameHandle = mpsc::Sender<(GameCommand, GameResp)>;
type GameRx = mpsc::Receiver<(GameCommand, GameResp)>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameInfo {
    game_id: GameId,
    group_ids: GameGroupIds,
    #[serde(skip)]
    game_handle: GameHandle,
}

struct Game {
    id: GameId,
    path: PathBuf,
    group_ids: GameGroupIds,
    game_rx: GameRx,
    state: State,
    event_rx: EventRx,
}

impl Game {
    pub fn create(id: GameId, path: PathBuf, group_ids: GameGroupIds, state: State) -> GameHandle {
        let (game_handle, game_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let game = Game { id, path, group_ids, game_rx, state, event_rx };
        game.start();
        game_handle
    }

    fn start(self) {
        tokio::spawn(self.run());
    }

    async fn run(self) {
        if !self.state.is_started() {
            self.state.start();
        }
        let mut deadline = self.state.update();
        self.handle_events().await;
        loop {
            let dur = Self::dur_until(deadline);
            match tokio::time::timeout(dur, self.game_rx.recv()).await {
                Ok(Some((cmd, tx))) => {
                    self.handle_cmd(cmd, tx).await;
                }
                Ok(None) => break, // Channel closed, end game
                Err(_) => {}       // Timeout, continue to update
            }
            deadline = self.state.update();
            self.handle_events().await;
        }
    }

    fn dur_until(deadline: Duration) -> Duration {
        let now = tokio::time::Instant::now();
        if now < deadline {
            deadline - now
        } else {
            Duration::from_secs(0)
        }
    }
    async fn handle_events(&mut self) {
        todo!()
    }

    async fn handle_cmd(&mut self, cmd: GameCommand, tx: GameResp) {
        todo!()
    }

    async fn save_state(&self) -> Result<()> {
        let state_path = self.path.join("state.json");
        let state_file = File::create(state_path).await?;
        serde_json::to_writer(state_file, &self.state)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameGroupIds {
    main: GroupId,
    mafia: GroupId,
    lobby: GroupId,
}

impl GameInfo {
    async fn create_game(
        members: Vec<groupme::Member>,
        rules: Rules,
        lobby_id: GroupId,
    ) -> Result<Self> {
        let players = members.iter().map(|m| W(m.user_id)).collect::<Vec<_>>();
        let state = State::new(players, rules);
        let game_id = GameId::new();
        let (game_handle, game_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create groups
        let main = api::create_group(&format!("Game {}", game_id), false).await?;
        let mafia = api::create_group(&format!("Mafia {}", game_id), false).await?;
        let group_ids = GameGroupIds { main: main.id, mafia: mafia.id, lobby: lobby_id };

        api::add_members(&main, members.clone()).await?;
        members.retain(|m| state.players().get_role(&W(m.user_id).into()).is_mafia());
        api::add_members(&mafia, members).await?;

        /// Hmm need base path
        state.event_tx = Some(event_tx);
        todo!()
        // let game_info = GameInfo { game_id, group_ids, game_handle };
    }

    async fn send(&self, cmd: GameCommand) -> GameResp {
        let (tx, mut rx) = oneshot::channel();
        self.game_handle.send((cmd, tx)).await.unwrap();
        rx.await.unwrap()
    }

    /// Load a game from a directory, and spawn its handler
    async fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut group_ids: Option<GameGroupIds> = None;
        let mut state: Option<State> = None;
        for entry in path.read_dir()? {
            let entry = entry?;
            let file_name = entry.file_name();
            match file_name.to_string_lossy() {
                "group_ids.json" => {
                    let file = File::open(entry.path()).await?;
                    group_ids = Some(serde_json::from_reader(file)?);
                }
                "state.json" => {
                    let file = File::open(entry.path()).await?;
                    state = Some(serde_json::from_reader(file)?);
                }
                _ => {}
            }
        }

        match (group_ids, state) {
            (Some(group_ids), Some(mut state)) => {
                let game_id = path.file_name().unwrap().to_string_lossy().to_owned().into();
                let (game_handle, game_rx) = mpsc::channel(1);
                let (event_tx, event_rx) = mpsc::unbounded_channel();
                state.event_tx = Some(event_tx);
                let game = Game {
                    id: game_id,
                    path: path.to_owned(),
                    group_ids: group_ids.clone(),
                    game_rx,
                    state,
                    event_rx,
                };
                let game_handle = game.start();
            }
            _ => bail!("Game directory is missing required files"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    Vote { user_id: W<UserId>, ballot: Option<Option<W<UserId>>> },
    Reveal { user_id: W<UserId> },
    Target { user_id: W<UserId>, target: Option<W<UserId>> },
    Scheme { user_id: W<UserId>, target: Option<W<UserId>> },
    Status,
}

impl RespContext {
    pub async fn send(&self, text: &str) -> Result<MessageId, groupme::api::Error> {
        match self {
            Self::Group(group_id, msg_id) => api::send_group_message(group_id, text).await,
            Self::User(user_id, msg_id) => api::send_dm(*user_id, text).await,
        }
    }
}

type Games = HashMap<GameId, GameInfo>;
type Lobbies = HashSet<GroupId>;
type Foci = HashMap<GroupId, GameId>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Controller {
    base_path: PathBuf,
    lobbies: Lobbies,
    games: Games,
    foci: Foci,
}

impl Controller {
    pub async fn find_games(&self) -> Result<Vec<GameInfo>> {
        let mut games = Vec::new();
        let games_path = self.base_path.join("games");
        for path in self.games_path.read_dir()? {
            match path {
                Ok(dir) if dir.file_type?.is_dir() => {
                    let game_id = dir.file_name().to_string_lossy().into_owned().into();
                    let game_path = self.base_path.join(&game_id);
                    // Load game from dir
                    let game_info = GameInfo::from_path(game_path).await?;
                }
                _ => {
                    panic!("Controller base path is not a directory!")
                }
            }
        }
        todo!()
    }
}
