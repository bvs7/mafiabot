use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::Duration,
};

use groupme::GroupId;
use mafia::game::GameId;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameInfo {
    main_chat: GroupId,
    mafia_chat: GroupId,
    lobby_chat: GroupId,
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

    pub async fn game_cmd(&self, game_id: GameId, cmd: ()) {}
}
