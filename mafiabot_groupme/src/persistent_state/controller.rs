use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc::TryRecvError, Arc},
    time::Duration,
};

use chrono::{DateTime, Local};
use mafia::{
    game::{self, ActionResp, Event2, GameId},
    state::{players, StateProc},
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
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameGroupIds {
    main: GroupId,
    mafia: GroupId,
    lobby: GroupId,
}

type GameResp = Resp<Result<ActionResp, GameError>>;

type GameTx = mpsc::Sender<(GameCommand, GameResp)>;
type GameRx = mpsc::Receiver<(GameCommand, GameResp)>;

#[derive(Debug, Clone)]
struct GameHandle {
    id: GameId,
    group_ids: GameGroupIds,
    game_tx: GameTx,
}

#[derive(Debug, Clone)]
struct Game {
    id: GameId,
    path: PathBuf,
    group_ids: GameGroupIds,
    state: State,
    rules: Rules,
}

impl Game {
    // How are games created?
    // New game creates a new state from members, rules...

    pub fn new(
        id: GameId,
        path: PathBuf,
        group_ids: GameGroupIds,
        state: State,
        rules: Rules,
    ) -> Self {
        Self { id, path, group_ids, state, rules }
    }

    /// Assumes members are already a part of groups...
    pub fn create(
        members: Vec<groupme::Member>,
        rules: Rules,
        lobby_id: GroupId,
        base_path: impl AsRef<Path>,
    ) -> Game {
        let id = GameId::new().unwrap();
        let path = base_path.as_ref().join(id.to_string());
        let players = members.iter().map(|m| W(m.user_id)).collect::<Vec<_>>();

        let state = State::new(players, rules.rolegen_config.clone());
        // Create group ids TODO based on roles
        let group_ids = GameGroupIds { main: GroupId(0), mafia: GroupId(1), lobby: lobby_id };
        Self::new(id, path, group_ids, state, rules)
    }

    pub async fn load(path: impl AsRef<Path>) -> Result<Game> {
        let path = path.as_ref();
        let game_id: GameId = path.file_name().unwrap().to_string_lossy().parse::<u64>()?.into();
        let mut group_ids: Option<GameGroupIds> = None;
        let mut state: Option<State> = None;
        let mut rules: Rules = Rules::default();
        for entry in path.read_dir()? {
            let entry = entry?;
            let file_name = entry.file_name();
            match file_name.to_string_lossy().as_ref() {
                "group_ids.json" => {
                    let mut file = File::open(entry.path()).await?;
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).await?;
                    group_ids = Some(serde_json::from_slice(&buf)?);
                }
                "state.json" => {
                    let mut file = File::open(entry.path()).await?;
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).await?;
                    state = Some(serde_json::from_slice(&buf)?);
                }
                "rules.json" => {
                    let mut file = File::open(entry.path()).await?;
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).await?;
                    rules = serde_json::from_slice(&buf)?;
                }
                _ => {}
            }
        }
        match (group_ids, state) {
            (Some(group_ids), Some(state)) => {
                Ok(Self::new(game_id, path.to_path_buf(), group_ids, state, rules))
            }
            _ => anyhow::bail!("Game directory is missing required files"),
        }
    }

    async fn save_state(&self) -> Result<()> {
        let state_path = self.path.join("state.json");
        let mut state_file = File::create(state_path).await?;
        let state_str = serde_json::to_string_pretty(&self.state)?;
        state_file.write_all(state_str.as_bytes()).await?;
        Ok(())
    }

    pub async fn save_group_ids(&self) -> Result<()> {
        let group_ids_path = self.path.join("group_ids.json");
        let mut group_ids_file = File::create(group_ids_path).await?;
        let group_ids_str = serde_json::to_string_pretty(&self.group_ids)?;
        group_ids_file.write_all(group_ids_str.as_bytes()).await?;
        Ok(())
    }

    pub async fn make_dir(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.path).await?;
        Ok(())
    }

    fn start(self) -> GameHandle {
        let (game_tx, game_rx) = mpsc::channel(1);
        let handle = GameHandle { id: self.id, group_ids: self.group_ids.clone(), game_tx };
        tokio::spawn(self.run(game_rx));
        handle
    }

    async fn run(mut self, mut game_rx: GameRx) {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        self.save_group_ids().await.unwrap();
        if !self.state.is_started() {
            self.state.start(&event_tx);
        }
        let mut deadline = self.state.update(&event_tx);
        self.handle_events(&mut event_rx).await;
        loop {
            let dur = Self::dur_until(deadline);
            match tokio::time::timeout(dur, game_rx.recv()).await {
                Ok(Some((cmd, tx))) => {
                    let resp = match cmd {
                        GameCommand::Action(action) => match self.state.validate_action(action) {
                            Ok(va) => Ok(self.state.perform_action(va, &event_tx)),
                            Err(err) => Err(err),
                        },
                        GameCommand::Status => {
                            // TODO more complicated responses!
                            Ok(ActionResp::Ok)
                        }
                    };
                }
                Ok(None) => break, // Channel closed, end game
                Err(_) => {}       // Timeout, continue to update
            }
            deadline = self.state.update(&event_tx);
            self.handle_events(&mut event_rx).await;
        }
    }

    fn dur_until(deadline: Option<DateTime<Local>>) -> Duration {
        match deadline.map(|d| (d - Local::now()).to_std()) {
            Some(Ok(dur)) => dur,
            Some(Err(_)) => Duration::ZERO,
            None => Duration::MAX,
        }
    }
    async fn handle_events(&mut self, event_rx: &mut EventRx) {
        while let Ok(event) = event_rx.try_recv() {
            let event = Event2::Debug(0);
            let _ = self.log_event(&event);
            match event {
                Event2::Start { players } => {}
                Event2::Day { day, players } => {}
                Event2::Reveal { player, role } => {}
                Event2::Night { day, players } => {}
                Event2::Eclipse { avenger, hammer, guilty } => {}
                Event2::Election { choice, hammer, vote_list } => {}
                Event2::Eliminate { player, role } => {}
                Event2::Dawn { night_actions } => {}
                Event2::Block { blocked, blockers } => {}
                Event2::Save { saved, saviors } => {}
                Event2::NoKill => {}
                Event2::Kill { killer, mark } => {}
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
        file.write_all(s.as_bytes()).await?;
        Ok(())
    }
}

impl GameHandle {
    async fn send(&self, cmd: GameCommand) -> Result<ActionResp, GameError> {
        let (tx, rx) = oneshot::channel();
        self.game_tx.send((cmd, tx)).await.unwrap();
        rx.await.unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum GameCommand {
    Action(Action<W<UserId>>),
    Status,
}

type Games = HashMap<GameId, GameHandle>;
type Lobbies = HashSet<GroupId>;
type Foci = HashMap<GroupId, GameId>;

#[derive(Debug, Clone, Default)]
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
        for path in games_path.read_dir()? {
            match path {
                Ok(dir) if dir.file_type()?.is_dir() => {
                    let game_id: GameId = dir.file_name().to_string_lossy().parse::<u64>()?.into();
                    let game_path = self.base_path.join(&game_id.to_string());
                    // Load game from dir
                    let game = Game::load(game_path).await?;
                    games.push(game);
                }
                _ => {
                    panic!("Controller base path is not a directory!")
                }
            }
        }
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use mafia::rolegen::RoleGenConfig;

    use super::*;

    #[tokio::test]
    async fn save_and_load() {
        let game_group_ids =
            GameGroupIds { main: GroupId(0), mafia: GroupId(1), lobby: GroupId(2) };
        let rolegen = RoleGenConfig::Debug(mafia::rolegen::DebugRoleGenConfig::new(vec![
            Role::TOWN,
            Role::TOWN,
            Role::MAFIA,
        ]));
        let state = State::new([1, 2, 3], rolegen);
        let cwd = std::env::current_dir().unwrap();
        let path = PathBuf::from("..").join("data").join("test_games").join("0");
        let game =
            Game::new(GameId::from(0), path.clone(), game_group_ids, state, Rules::default());

        game.save_group_ids().await.unwrap();
        game.save_state().await.unwrap();

        let game2 = Game::load(path).await.unwrap();
        println!("{:?}", game2);
    }
}
