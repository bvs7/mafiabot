use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc::TryRecvError, Arc},
    time::Duration,
};

use chrono::{DateTime, Local};
use mafia::{
    game::{self, Event2, GameId},
    state::{
        action::{Action, ActionResp, Validated},
        players, StateProc,
    },
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
};

use crate::prelude::*;

use mafia::state::action::Command as GameCommand;

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

type GameTx = mpsc::Sender<(GameCommand<W<UserId>>, GameResp)>;
type GameRx = mpsc::Receiver<(GameCommand<W<UserId>>, GameResp)>;

#[derive(Debug)]
struct GameHandle {
    id: GameId,
    group_ids: GameGroupIds,
    game_tx: GameTx,
    handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct Game {
    id: GameId,
    group_ids: GameGroupIds,
    state: State,
    rules: Rules,
}

impl Game {
    // How are games created?
    // New game creates a new state from members, rules...

    pub fn new(id: GameId, group_ids: GameGroupIds, state: State, rules: Rules) -> Self {
        Self { id, group_ids, state, rules }
    }

    /// Assumes members are already a part of groups...
    pub fn create(
        members: Vec<groupme::Member>,
        rules: Rules,
        lobby_id: GroupId,
        base_path: impl AsRef<Path>,
    ) -> Game {
        let id = GameId::new().unwrap();
        let players = members.iter().map(|m| W(m.user_id)).collect::<Vec<_>>();

        let state = State::new(players, rules.rolegen_config.clone());
        // Create group ids TODO based on roles
        let group_ids = GameGroupIds { main: GroupId(0), mafia: GroupId(1), lobby: lobby_id };
        Self::new(id, group_ids, state, rules)
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
            (Some(group_ids), Some(state)) => Ok(Self::new(game_id, group_ids, state, rules)),
            _ => anyhow::bail!("Game directory is missing required files"),
        }
    }
    pub async fn save_group_ids(&self, base_path: impl AsRef<Path>) -> Result<()> {
        let group_ids_path = self.game_dir(base_path).join("group_ids.json");
        let mut group_ids_file = File::create(group_ids_path).await?;
        let group_ids_str = serde_json::to_string_pretty(&self.group_ids)?;
        group_ids_file.write_all(group_ids_str.as_bytes()).await?;
        Ok(())
    }

    async fn save_state(&self, base_path: impl AsRef<Path>) -> Result<()> {
        let state_path = self.game_dir(base_path).join("state.json");
        let mut state_file = File::create(state_path).await?;
        let state_str = serde_json::to_string_pretty(&self.state)?;
        state_file.write_all(state_str.as_bytes()).await?;
        Ok(())
    }

    async fn save_rules(&self, base_path: impl AsRef<Path>) -> Result<()> {
        let rules_path = self.game_dir(base_path).join("rules.json");
        let mut rules_file = File::create(rules_path).await?;
        let rules_str = serde_json::to_string_pretty(&self.rules)?;
        rules_file.write_all(rules_str.as_bytes()).await?;
        Ok(())
    }

    pub async fn save_game(&self, base_path: impl AsRef<Path>) -> Result<()> {
        tokio::fs::create_dir_all(self.game_dir(&base_path)).await?;
        self.save_group_ids(&base_path).await?;
        self.save_state(&base_path).await?;
        self.save_rules(&base_path).await?;
        Ok(())
    }
    fn game_dir(&self, base_path: impl AsRef<Path>) -> PathBuf {
        base_path.as_ref().join("games").join(self.id.to_string())
    }

    fn start(self, base_path: impl AsRef<Path>) -> GameHandle {
        debug!("Start?");
        let id = self.id;
        let group_ids = self.group_ids.clone();
        let (game_tx, game_rx) = mpsc::channel(1);
        let handle = tokio::spawn(self.run(game_rx, base_path.as_ref().to_owned()));
        GameHandle { id, group_ids, game_tx, handle }
    }

    #[tracing::instrument(skip_all)]
    async fn run(mut self, mut game_rx: GameRx, base_path: PathBuf) {
        debug!("Start Run");
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        self.save_group_ids(&base_path).await.unwrap();
        if !self.state.is_started() {
            self.state.start(&event_tx);
        }
        let mut deadline = self.state.update(&event_tx);
        self.save_state(&base_path).await.unwrap();
        self.handle_events(&mut event_rx, &base_path).await;
        while !self.state.is_ended() {
            let dur = Self::dur_until(deadline);
            match tokio::time::timeout(dur, game_rx.recv()).await {
                Ok(Some((cmd, tx))) => {
                    let resp = match self.state.validate_command(cmd) {
                        Ok(Validated::Action(action)) => {
                            self.log_action(&action, &base_path).await.unwrap();
                            Ok(self.state.perform_action(action, &event_tx))
                        }
                        Ok(Validated::Resp(resp)) => Ok(resp),
                        Err(err) => {
                            let _ = tx.send(Err(err));
                            continue;
                        }
                    };
                    let _ = tx.send(resp);
                }
                Ok(None) => break, // Channel closed, end game
                Err(_) => {
                    // Timeout, continue to update
                    info!("Timeout");
                } // Timeout, continue to update
            }

            deadline = self.state.update(&event_tx);
            self.save_state(&base_path).await.unwrap();
            self.handle_events(&mut event_rx, &base_path).await;
        }
    }

    fn dur_until(deadline: Option<DateTime<Local>>) -> Duration {
        match deadline.map(|d| (d - Local::now()).to_std()) {
            Some(Ok(dur)) => dur,

            Some(Err(_)) => Duration::ZERO,
            None => Duration::MAX,
        }
    }

    async fn log_action(&self, action: &Action, base_path: impl AsRef<Path>) -> Result<()> {
        let action_path = self.game_dir(base_path).join("action.log");
        let mut file = OpenOptions::new().append(true).create(true).open(action_path).await?;
        let s = format!("{}\n", action);
        file.write_all(s.as_bytes()).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn handle_events(&mut self, event_rx: &mut EventRx, base_path: impl AsRef<Path>) {
        while let Ok(event) = event_rx.try_recv() {
            debug!("{:?}", event);
            let _ = self.log_event(&event, &base_path).await;
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
    async fn log_event(&mut self, event: &Event2, base_path: impl AsRef<Path>) -> Result<()> {
        let event_path = self.game_dir(base_path).join("event.log");
        let mut file = OpenOptions::new().append(true).create(true).open(event_path).await?;
        let s = format!("{}\n", event);
        file.write_all(s.as_bytes()).await?;
        Ok(())
    }
}

impl GameHandle {
    pub async fn send(&self, cmd: GameCommand<W<UserId>>) -> Result<ActionResp, GameError> {
        let (tx, rx) = oneshot::channel();
        self.game_tx.send((cmd, tx)).await.unwrap();
        rx.await.unwrap()
    }
}

type Games = HashMap<GameId, GameHandle>;
type Lobbies = HashSet<GroupId>;
type Foci = HashMap<GroupId, GameId>;

#[derive(Debug, Default)]
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
        let mut rules = Rules::default();
        rules.debug = Some(10);
        rules.rolegen_config = rolegen.clone();
        let state = State::new([1, 2, 3], rolegen);
        let cwd = std::env::current_dir().unwrap();
        let path = PathBuf::from("..").join("data").join("test_games");
        let game = Game::new(GameId::from(0), game_group_ids, state, rules);

        game.save_game(&path).await.unwrap();

        let game2 = Game::load(path.join("games").join("0")).await.unwrap();
        println!("{:?}", game2);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn load_and_run() {
        std::env::set_var("RUST_LOG", "debug");
        let base_path = PathBuf::from("..").join("data").join("test_games");
        let game_path = base_path.join("games").join("0");
        let game = Game::load(&game_path).await.unwrap();
        let game_handle = Game::start(game, base_path.clone());
        let r = game_handle
            .send(GameCommand::Vote { voter: W(UserId(1)), ballot: Some(Some(W(UserId(2)))) })
            .await;

        let r = game_handle
            .send(GameCommand::Vote { voter: W(UserId(3)), ballot: Some(Some(W(UserId(2)))) })
            .await;

        tokio::time::sleep(Duration::from_millis(2000)).await;

        game_handle.handle.abort();

        // let game = Game::load(&game_path).await.unwrap();
        // let game_handle = Game::start(game, base_path.clone());
        // tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}
