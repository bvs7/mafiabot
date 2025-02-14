use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{mpsc::TryRecvError, Arc},
    time::Duration,
};

use chrono::{DateTime, Local};
use mafia::{
    game::{self, Error as GameError, Event2, GameId},
    state::{
        action::{ActionResp, ValidAction, Validated},
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

use mafia::state::action::Action;

#[derive(Debug)]
pub enum Command {
    Action(Action<W<UserId>>, Resp<Result<ActionResp, GameError>>),
    Status(Resp<State>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameGroupIds {
    main: GroupId,
    mafia: GroupId,
    lobby: GroupId,
}

type GameTx = mpsc::Sender<Command>;
type GameRx = mpsc::Receiver<Command>;

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
        loop {
            let dur = Self::dur_until(deadline);
            match tokio::time::timeout(dur, game_rx.recv()).await {
                Ok(Some(Command::Action(action, resp))) => {
                    let _ = resp.send(match self.state.validate_action(action) {
                        Ok(va) => {
                            self.log_action(&va, &base_path).await.unwrap();
                            Ok(self.state.perform_action(va, &event_tx))
                        }
                        Err(err) => Err(err),
                    });
                }
                Ok(Some(Command::Status(resp))) => {
                    let status = self.state.clone();
                    let _ = resp.send(status);
                }
                Ok(None) => break, // Channel closed, end game
                Err(_) => {
                    // Timeout, continue to update
                    info!("Timeout");
                }
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

    async fn log_action(&self, action: &ValidAction, base_path: impl AsRef<Path>) -> Result<()> {
        let action_path = self.game_dir(base_path).join("action.log");
        let mut file = OpenOptions::new().append(true).create(true).open(action_path).await?;
        // let s = format!("{}\n", action);
        let s = serde_json::to_string(action)?;
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
    pub async fn send_action(&self, action: Action<W<UserId>>) -> Result<ActionResp, GameError> {
        let (tx, rx) = oneshot::channel();
        let cmd = Command::Action(action, tx);
        self.game_tx.send(cmd).await.unwrap();
        rx.await.unwrap()
    }

    pub async fn get_status(&self) -> State {
        let (tx, rx) = oneshot::channel();
        let cmd = Command::Status(tx);
        self.game_tx.send(cmd).await.unwrap();
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

    use mafia::{
        rolegen::{self, DebugRoleGenConfig, RoleGenConfig},
        state::phase::Phase,
    };

    use super::*;

    fn base_path() -> PathBuf {
        PathBuf::from("../data/test_games")
    }

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
        let v1 = Action::Vote { voter: W(UserId(1)), ballot: Some(Some(W(UserId(2)))) };
        let v2 = Action::Vote { voter: W(UserId(3)), ballot: Some(Some(W(UserId(2)))) };
        game_handle.send_action(v1).await.unwrap();
        game_handle.send_action(v2).await.unwrap();

        tokio::time::sleep(Duration::from_millis(2000)).await;

        // let game = Game::load(&game_path).await.unwrap();
        // let game_handle = Game::start(game, base_path.clone());
        // tokio::time::sleep(Duration::from_millis(2000)).await;
    }

    #[tokio::test]
    async fn basic_game() {
        let game_group_ids =
            GameGroupIds { main: GroupId(0), mafia: GroupId(1), lobby: GroupId(2) };
        let rolegen = RoleGenConfig::Debug(DebugRoleGenConfig::new(vec![
            Role::TOWN,
            Role::TOWN,
            Role::MAFIA,
        ]));
        let mut rules = Rules::default();
        rules.debug = Some(10);
        let state = State::new([1, 2, 3], rolegen);
        let game = Game::new(GameId::from(0), game_group_ids, state, rules);

        let handle = game.start(base_path());

        handle
            .send_action(Action::Vote { voter: W(UserId(1)), ballot: Some(Some(W(UserId(2)))) })
            .await
            .unwrap();
        handle.send_action(Action::Vote { voter: W(UserId(1)), ballot: Some(None) }).await.unwrap();
        handle.send_action(Action::Vote { voter: W(UserId(2)), ballot: Some(None) }).await.unwrap();

        let state = handle.get_status().await;
        assert!(matches!(state.phase, Phase::Day { .. }));
        tokio::time::sleep(Duration::from_secs(2)).await;

        let state = handle.get_status().await;
        assert!(matches!(state.phase, Phase::Night { .. }));

        handle.send_action(Action::Scheme { killer: W(UserId(3)), mark: None }).await.unwrap();

        let state = handle.get_status().await;
        assert!(matches!(state.phase, Phase::Night { .. }));

        tokio::time::sleep(Duration::from_secs(2)).await;

        let state = handle.get_status().await;
        assert!(matches!(state.phase, Phase::Day { .. }));

        handle.send_action(Action::Vote { voter: W(UserId(1)), ballot: Some(None) }).await.unwrap();
        handle.send_action(Action::Vote { voter: W(UserId(2)), ballot: Some(None) }).await.unwrap();
        handle.send_action(Action::Vote { voter: W(UserId(1)), ballot: None }).await.unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;

        let state = handle.get_status().await;
        assert!(matches!(state.phase, Phase::Day { .. }));

        handle
            .send_action(Action::Vote { voter: W(UserId(1)), ballot: Some(Some(W(UserId(3)))) })
            .await
            .unwrap();
        handle
            .send_action(Action::Vote { voter: W(UserId(2)), ballot: Some(Some(W(UserId(3)))) })
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;

        let state = handle.get_status().await;
        assert!(state.is_ended());
    }
}
