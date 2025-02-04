use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc::TryRecvError, Arc},
    time::Duration,
};

use chrono::{DateTime, Local};
use mafia::{
    game::{self, ActionResp, Event2, GameId},
    state::players,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::prelude::*;

// TODO: flesh out Game and GameHandle?
/*
// Creating or loading a game should return a GameInfo with a running Game under the hood
// Games can be created either...
// A new game, from Members, rules, lobby_id, and base_path. Maybe this should be done from Controller?
// Automatically starting the game seems fine... but let's just not for now.

// Or from a dir. Which has state.json and game_group_ids.json, (and later event.log and other things?)
// Does state need rules? If not that could be another file maybe...

*/

type GameResp = Resp<Result<ActionResp, GameError>>;

type GameTx = mpsc::Sender<(GameCommand, GameResp)>;
type GameRx = mpsc::Receiver<(GameCommand, GameResp)>;

#[derive(Debug, Clone)]
struct GameHandle {
    id: GameId,
    group_ids: GameGroupIds,
    game_tx: GameTx,
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
    // How are games created?
    // New game creates a new state from members, rules...

    /// Assumes members are already a part of groups...
    pub fn create(
        id: GameId,
        path: impl AsRef<Path>,
        group_ids: GameGroupIds,
        state: State,
    ) -> GameHandle {
        let (game_tx, game_rx) = mpsc::channel(1);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let path = path.as_ref().to_owned();
        let game = Game { id, path, group_ids: group_ids.clone(), game_rx, state, event_rx };
        game.start();
        GameHandle { id, group_ids, game_tx }
    }

    pub async fn load(game_id: GameId, path: impl AsRef<Path>) -> Result<GameHandle> {
        let path = path.as_ref();
        let mut group_ids: Option<GameGroupIds> = None;
        let mut state: Option<State> = None;
        for entry in path.read_dir()? {
            let entry = entry?;
            let file_name = entry.file_name();
            match file_name.to_string_lossy().as_ref() {
                "group_ids.json" => {
                    let mut file = File::open(entry.path()).await?;
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf);
                    group_ids = Some(serde_json::from_slice(&buf)?);
                }
                "state.json" => {
                    let mut file = File::open(entry.path()).await?;
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf);
                    state = Some(serde_json::from_slice(&buf)?);
                }
                _ => {}
            }
        }
        match (group_ids, state) {
            (Some(group_ids), Some(state)) => {
                return Ok(Self::create(game_id, path, group_ids, state));
            }
            _ => anyhow::bail!("Game directory is missing required files"),
        }
    }

    fn start(self) {
        tokio::spawn(self.run());
    }

    async fn run(mut self) {
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

    fn dur_until(deadline: Option<DateTime<Local>>) -> Duration {
        match deadline.map(|d| (d - Local::now()).to_std()) {
            Some(Ok(dur)) => dur,
            Some(Err(_)) => Duration::ZERO,
            None => Duration::MAX,
        }
    }
    async fn handle_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            let event = Event2::Debug(0);
            let _ = self.log_event(&event);
            match event {
                Event2::Start { players, rules } => {}
                Event2::Day { day, players } => {}
                Event2::Reveal { celeb } => {}
                Event2::Night { day, players } => {}
                Event2::Eclipse { avenger, hammer, guilty } => {}
                Event2::Election { choice, hammer, vote_list } => {}
                Event2::Eliminate { player, role } => {}
                Event2::Dawn { night_actions } => {}
                Event2::Block { blocked, blockers } => {}
                Event2::Save { saved, saviors } => {}
                Event2::NoKill => {}
                Event2::Kill { actor, target } => {}
                Event2::Investigate { cop, target, appears_mafia } => {}
                Event2::Milk { milky, target } => {}
                Event2::End { winner } => {}
                Event2::Debug(n) => {}
            }
        }
    }
    async fn log_event(&mut self, event: &Event2) -> Result<()> {
        let event_path = self.path.join("event.log");
        let mut file = OpenOptions::new().append(true).create(true).open(event_path).await?;
        let s = format!("{:?}\n", event);
        file.write_all(s.as_bytes());
        Ok(())
    }

    async fn handle_cmd(&mut self, cmd: GameCommand, tx: GameResp) {
        match cmd {
            GameCommand::Action(action) => {
                let resp = match self.state.validate_action(action) {
                    Ok(va) => self.state.perform_action(va),
                    Err(err) => Err(err),
                };
                let _ = tx.send(resp);
            }
            GameCommand::Status => {
                // TODO more complicated responses!
                let resp = Ok(ActionResp::Ok);
                let _ = tx.send(resp);
            }
        }
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

impl GameHandle {
    async fn send(&self, cmd: GameCommand) -> GameResp {
        let (tx, mut rx) = oneshot::channel();
        self.game_handle.send((cmd, tx)).await.unwrap();
        rx.await.unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    Action(Action<W<UserId>>),
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

type Games = HashMap<GameId, GameHandle>;
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
    pub async fn find_games(&self) -> Result<Vec<GameHandle>> {
        let mut games = Vec::new();
        let games_path = self.base_path.join("games");
        for path in self.games_path.read_dir()? {
            match path {
                Ok(dir) if dir.file_type?.is_dir() => {
                    let game_id = dir.file_name().to_string_lossy().into_owned().into();
                    let game_path = self.base_path.join(&game_id);
                    // Load game from dir
                    let game_info = GameHandle::from_path(game_path).await?;
                }
                _ => {
                    panic!("Controller base path is not a directory!")
                }
            }
        }
        todo!()
    }
}
