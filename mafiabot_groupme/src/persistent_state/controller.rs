use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
    time::Duration,
};

use mafia::game::GameId;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameInfo {
    main_chat: GroupId,
    mafia_chat: GroupId,
    lobby_chat: GroupId,
    #[serde(skip)]
    lock: Arc<tokio::sync::RwLock<()>>,
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    Vote { user_id: W<UserId>, ballot: Option<Option<W<UserId>>> },
    Reveal { user_id: W<UserId> },
    Target { user_id: W<UserId>, target: Option<W<UserId>> },
    Scheme { user_id: W<UserId>, target: Option<W<UserId>> },
    Status,
}

#[derive(Debug, Clone)]
pub enum RespContext {
    Group(GroupId, MessageId),
    User(UserId, MessageId),
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
    base_path: String,
    lobbies: Lobbies,
    games: Games,
    foci: Foci,
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path).await?;
        let mut buf = Vec::new();
        let result =
            tokio::time::timeout(Duration::from_secs(5), file.read_to_end(&mut buf)).await??;
        Ok(serde_json::from_slice(&buf)?)
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::open(path).await?;
        let ctrl_json = serde_json::to_vec(self)?;
        file.write_all(&ctrl_json).await?;
        Ok(())
    }

    pub async fn game_cmd(
        &self,
        game_id: GameId,
        cmd: GameCommand,
        resp: RespContext,
    ) -> Result<()> {
        let game_info = self.games.get(&game_id).unwrap();
        // TODO: read vs write
        let _lock = game_info.lock.write().await;
        let game_dir = Path::new(&self.base_path).join(game_id.to_string());
        if !game_dir.is_dir() {
            anyhow::bail!("Game directory not found");
        }
        let state_path = game_dir.join("state.json");
        let mut state = State::load(&state_path).await?;
        let (tx, rx) = mpsc::unbounded_channel::<Event>();
        let _ = state.event_tx.insert(tx);

        let action = match cmd {
            GameCommand::Vote { user_id: voter, ballot } => Some(Action::Vote { voter, ballot }),
            GameCommand::Reveal { user_id: actor } => Some(Action::Reveal { actor }),
            GameCommand::Target { user_id: actor, target: choice } => {
                Some(Action::Target { actor, choice })
            }
            GameCommand::Scheme { user_id: killer, target: mark } => {
                Some(Action::Scheme { killer, mark })
            }
            GameCommand::Status => None,
        };
        if let Some(action) = action {
            match state.validate_action(action) {
                Ok(va) => {
                    state.perform_action(va);
                }
                Err(e) => {
                    resp.send(&e.to_string()).await?;
                }
            }
        } else {
            resp.send(&format!("{}", state)).await?;
        }

        // Now, if state changed..
        state.save(&state_path).await?;
        drop(_lock);
        Ok(())
    }
}
