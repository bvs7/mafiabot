// use broadcast::error::RecvError;
// use rand::thread_rng;

use std::{collections::HashSet, time::Duration};

use crate::prelude::*;

mod app_state;
pub use app_state::AppState;
use chrono::{format, DateTime, Local};
use groupme::Member;
use tokio::{
    task::JoinHandle,
    time::{error::Elapsed, Instant, Timeout},
};
mod parse;

struct Brief {
    game_id: GameId,
    day: u32,
    phase: PhaseKind,
    counts: HashMap<Team, usize>,
}

impl std::fmt::Display for Brief {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Game {}: {} {}", self.game_id, self.phase, self.day)?;
        if let Some(count) = self.counts.get(&Team::Town) {
            write!(f, "\n  Town: {}", count)?;
        }
        if let Some(count) = self.counts.get(&Team::Mafia) {
            write!(f, "\n  Mafia: {}", count)?;
        }
        if let Some(count) = self.counts.get(&Team::Rogue) {
            write!(f, "\n  Rogue: {}", count)?;
        }
        Ok(())
    }
}

enum GameMessage {
    SendAction { action: Action<u64>, resp: Resp<Result<(), GameError>> },
    GetStatus { resp: Resp<State> },
    GetBrief { resp: Resp<Brief> },
    GetTarget { target_ascii: u8, resp: Resp<Result<Option<u64>, u8>> },
}

type Resp<T> = oneshot::Sender<T>;

enum ControllerMessge {
    CreateGame { members: Vec<Member>, rules: Rules, response: Resp<GameHandle> },
    DestroyGame { game_id: GameId },
    CreateLobby {},
    Message { data: Data },
}

#[derive(Debug, Clone)]
struct GameHandle {
    game_id: GameId,
    main_chat_id: GroupId,
    mafia_chat_id: GroupId,
    tx: mpsc::Sender<GameMessage>,
}

impl GameHandle {
    pub fn id(&self) -> GameId {
        self.game_id
    }
    pub async fn get_target(&self, target_ascii: u8) -> Result<Option<u64>, u8> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(GameMessage::GetTarget { target_ascii, resp: tx })
            .await
            .expect("Game should receive");
        rx.await.expect("Game shouldn't drop tx")
    }
    pub async fn send_action(&self, action: Action<u64>) -> Result<(), GameError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(GameMessage::SendAction { action, resp: tx })
            .await
            .expect("Game should receive");
        rx.await.expect("Game shouldn't drop tx")
    }
    pub async fn get_status(&self) -> State {
        let (tx, rx) = oneshot::channel();
        self.tx.send(GameMessage::GetStatus { resp: tx }).await.expect("Game should receive");
        rx.await.expect("Game shouldn't drop tx")
    }
    pub async fn get_brief(&self) -> Brief {
        let (tx, rx) = oneshot::channel();
        self.tx.send(GameMessage::GetBrief { resp: tx }).await.expect("Game should receive");
        rx.await.expect("Game shouldn't drop tx")
    }
}

// enum LobbyMessage {}

struct Lobby {
    lobby_chat: groupme::Group,
    games: watch::Sender<HashMap<GameId, GameHandle>>,
    start_msg: watch::Receiver<Option<(MessageId, usize, Instant)>>,
    rules: Rules,
    ctrl_handle: ControllerHandle,
}

impl Lobby {
    pub fn new() -> Self {
        todo!()
    }
    pub fn from_group(group: groupme::Group) -> Self {
        todo!()
    }

    async fn run(mut self) -> JoinHandle<()> {
        loop {
            let start_msg = self.start_msg.borrow_and_update().clone();
            if let Some((msg_id, min_players, start_time)) = start_msg {
                match tokio::time::timeout_at(start_time, self.start_msg.changed()).await {
                    Err(Elapsed) => {
                        self.start_game(msg_id, min_players).await;
                    }
                    Ok(_) => {}
                }
            }
        }
    }

    async fn start_game(&mut self, msg_id: MessageId, min_players: usize) {
        let lobby_id = self.lobby_chat.id();
        let msg = api::get_group_message(&lobby_id, &msg_id).await.unwrap();
        let users = msg.favorited_by;
        self.lobby_chat.update_names().await;
        let members = users
            .into_iter()
            .map(|u_id| {
                let name = self.lobby_chat.name(&u_id).unwrap_or_else(|| format!("User {}", u_id));
                Member::new(name, u_id)
            })
            .collect();
        let game_handle = self.ctrl_handle.start_game(members, self.rules.clone()).await;
        self.games.send_modify(|games| {
            games.insert(game_handle.id(), game_handle);
        });
    }
}

#[derive(Debug, Clone)]
struct LobbyHandle {
    lobby_chat_id: GroupId,
    games: watch::Receiver<HashMap<GameId, GameHandle>>, // TODO should this be internal? A watch?
    start_msg: watch::Sender<Option<(MessageId, usize, Instant)>>,
}

impl LobbyHandle {
    pub async fn send_start_msg(&self, minutes: u64, min_players: usize) {
        let msg =
            format!("Starting a game in {} minutes with at least {} players", minutes, min_players);
        let msg_id =
            api::send_group_message(&self.lobby_chat_id, &msg).await.expect("Message should send");
        let start_time = Instant::now() + Duration::from_secs(minutes * 60);
        self.start_msg.send(Some((msg_id, min_players, start_time))).expect("Lobby should receive");
    }

    async fn perform_lobby_cmd(&self, cmd: LobbyCommand, resp: RespContext) {
        use LobbyCommand::*;
        match cmd {
            Start { minutes, min_players } => self.send_start_msg(minutes, min_players).await,
            Status => {
                let mut msg = String::new();
                let games = self.games.borrow();
                if games.is_empty() {
                    msg.push_str("No games in lobby");
                } else {
                    msg.push_str("Games in lobby:");
                    for game in games.values() {
                        let brief = game.get_brief().await;
                        msg.push_str(&format!("\n{}", brief));
                    }
                }
                let _ = api::send_group_message(&self.lobby_chat_id, &msg).await;
            }
            StatusOf { game_id } => {
                let games = self.games.borrow();
                let Some(game) = games.get(&game_id) else {
                    let _ = api::send_group_message(&self.lobby_chat_id, "Game not found").await;
                    return;
                };
                let brief = game.get_brief().await;
                let msg = format!("{}", brief);
                let _ = api::send_group_message(&self.lobby_chat_id, &msg).await;
            }
        }
    }
}

struct Controller {
    rx: mpsc::Receiver<ControllerMessge>,
    lobbies: watch::Sender<HashMap<GroupId, LobbyHandle>>,
    games: watch::Sender<HashMap<GameId, GameHandle>>,
    focus: watch::Sender<HashMap<UserId, GameId>>,
}

impl Controller {
    async fn handle_msg(&mut self) {
        loop {
            use ControllerMessge::*;
            match self.rx.recv().await {
                Some(CreateGame { members, rules, response }) => {
                    todo!()
                }
                Some(DestroyGame { game_id }) => {
                    todo!()
                }
                Some(CreateLobby {}) => {
                    todo!()
                }
                Some(Message { data }) => {
                    todo!()
                }
                None => break,
            }
        }
    }
}

struct ControllerHandle {
    tx: mpsc::Sender<ControllerMessge>,
    lobbies: watch::Receiver<HashMap<GroupId, LobbyHandle>>,
    games: watch::Receiver<HashMap<GameId, GameHandle>>,
    focus: watch::Receiver<HashMap<UserId, GameId>>,
}

impl ControllerHandle {
    pub async fn start_game(&self, members: Vec<Member>, rules: Rules) -> GameHandle {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ControllerMessge::CreateGame { members, rules, response: tx })
            .await
            .expect("Controller should receive");
        rx.await.expect("Controller shouldn't drop tx")
    }
}

#[derive(Debug, Clone)]
enum Command {
    Lobby(GroupId, LobbyCommand),
    Game(GameId, GameCommand),
    App(UserId, AppCommand),
}

#[derive(Debug, Clone)]
enum LobbyCommand {
    Start { minutes: u64, min_players: usize },
    Status,
    StatusOf { game_id: GameId },
}

#[derive(Debug, Clone)]
enum GameCommand {
    Vote { user_id: W<UserId>, ballot: Option<Option<W<UserId>>> },
    Reveal { user_id: W<UserId> },
    Target { user_id: W<UserId>, target: Option<W<UserId>> },
    Scheme { user_id: W<UserId>, target: Option<W<UserId>> },
    Status,
}

#[derive(Debug, Clone)]
enum AppCommand {
    GetFocus,
    Focus { game_id: GameId },
}

enum RespContext {
    Group(GroupId),
    User(UserId),
}

type Parse<T> = Result<T, String>;

impl ControllerHandle {
    async fn perform_cmd(&mut self, cmd: Command, resp: RespContext) {
        match cmd {
            Command::Lobby(group_id, cmd) => {
                let lobbies = self.lobbies.borrow();
                let lobby = lobbies.get(&group_id).expect("Lobby should exist");
                lobby.perform_lobby_cmd(cmd, resp).await;
            }
            Command::Game(game_id, cmd) => {
                let games = self.games.borrow();
                let game = games.get(&game_id).expect("Game should exist");
            }
            Command::App(user_id, cmd) => match cmd {
                AppCommand::GetFocus => {
                    todo!()
                }
                AppCommand::Focus { game_id } => {
                    todo!()
                }
            },
        }
    }

    async fn parse_cmd(&self, data: Data) -> Option<(Parse<Command>, RespContext)> {
        let text = data.text();
        let mut chars = text.chars();
        let Some('/') = chars.next() else {
            return None;
        };
        let text: String = chars.collect();
        match data {
            Data::GroupMsg { group_id, user_id, attachments, .. } => {
                if let Some(cmd) = self.parse_group_cmd(&group_id, user_id, text, attachments).await
                {
                    return Some((cmd, RespContext::Group(group_id)));
                }
            }
            Data::DirectMsg { attachments, id, name, text, user_id, .. } => {
                todo!()
            }
            Data::Unknown => {
                todo!()
            }
        }
        None
    }

    async fn parse_group_cmd(
        &self,
        group_id: &GroupId,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Option<Parse<Command>> {
        // Check for a lobby
        let lobbies = self.lobbies.borrow();
        let games = self.games.borrow();
        let cmd = if let Some(lobby) = lobbies.get(group_id) {
            self.parse_lobby_cmd(lobby, user_id, text, attachments)
                .await
                .map(|p| p.map(|cmd| Command::Lobby(*group_id, cmd)))
        } else if let Some((game_id, game)) =
            games.iter().find(|(g_id, g)| &g.main_chat_id == group_id)
        {
            self.parse_main_chat_cmd(user_id, &text, attachments)
                .await
                .map(|p| p.map(|cmd| Command::Game(*game_id, cmd)))
        } else if let Some((game_id, game)) =
            games.iter().find(|(g_id, g)| &g.mafia_chat_id == group_id)
        {
            self.parse_mafia_chat_cmd(game, user_id, &text)
                .await
                .map(|p| p.map(|cmd| Command::Game(*game_id, cmd)))
        } else {
            None
        };
        todo!("Parse an app command")
    }

    async fn parse_lobby_cmd(
        &self,
        lobby: &LobbyHandle,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Option<Parse<LobbyCommand>> {
        use LobbyCommand::*;
        let user_id = W(user_id);
        let mut words = text.split_whitespace();
        let cmd = match (words.next(), words.next(), words.next()) {
            (Some("start"), mins, min_ps) => {
                let mut minutes = 10;
                if let Some(Ok(m)) = mins.map(|s| s.parse::<u64>()) {
                    if minutes <= 120 {
                        minutes = m;
                    }
                }
                let mut min_players = 5_usize;
                if let Some(Ok(m)) = min_ps.map(|s| s.parse::<usize>()) {
                    min_players = m;
                }
                Start { minutes, min_players }
            }
            (Some("status"), Some(game_id), _) => {
                let Some(game_id) = game_id.parse::<u64>().ok().map(GameId::from) else {
                    return Some(Err(format!("Could not parse game id: {}", game_id)));
                };
                StatusOf { game_id }
            }
            (Some("status"), None, _) => Status,
            _ => return None,
        };
        Some(Ok(cmd))
    }

    async fn parse_main_chat_cmd(
        &self,
        user_id: UserId,
        text: &str,
        attachments: Vec<Attachment>,
    ) -> Option<Parse<GameCommand>> {
        let words = text.split_whitespace().collect::<Vec<&str>>();
        let mentions = attachments.into_iter().find(|a| matches!(a, Attachment::Mentions { .. }));
        use Attachment::Mentions;
        use GameCommand::*;
        let user_id = W(user_id);
        let cmd = match words[..] {
            ["vote", "nokill", ..] | ["vote", "none", ..] => Vote { user_id, ballot: Some(None) },
            ["unvote", ..] => Vote { user_id, ballot: None },
            ["vote", ..] => {
                if let Some(Mentions { user_ids }) = mentions {
                    let Some(other) = user_ids.first() else {
                        return Some(Err(format!("Could not parse mentions")));
                    };
                    let ballot = Some(Some(W(UserId(*other))));
                    Vote { user_id, ballot }
                } else {
                    Vote { user_id, ballot: None }
                }
            }
            ["status", ..] => Status,
            _ => return None,
        };
        Some(Ok(cmd))
    }

    async fn parse_mafia_chat_cmd(
        &self,
        game: &GameHandle,
        user_id: UserId,
        text: &str,
    ) -> Option<Parse<GameCommand>> {
        let words = text.split_whitespace().collect::<Vec<&str>>();
        use GameCommand::*;
        let user_id = W(user_id);
        let cmd = match words[..] {
            ["target", target, ..] => {
                // match implies target is not empty
                let bytes = target.as_bytes();
                let [c] = bytes[..] else {
                    return Some(Err(format!("Could not parse target {}", target)));
                };
                let Ok(target_id) = game.get_target(c).await else {
                    return Some(Err(format!("Could not find target {}", target)));
                };
                let target = target_id.map(|t| W(UserId(t)));
                Scheme { user_id, target }
            }
            _ => return None,
        };
        Some(Ok(cmd))
    }

    async fn parse_dm_cmd(&self, user_id: UserId, text: String) -> Option<Parse<Command>> {
        let focus = self.focus.borrow();
        let focus = focus.get(&user_id).copied();
        let words = text.split_whitespace().collect::<Vec<&str>>();
        let cmd = match words[..] {
            ["focus", game_id, ..] => {
                let Some(game_id) = game_id.parse::<u64>().ok().map(GameId::from) else {
                    return Some(Err(format!("Could not parse game id: {}", game_id)));
                };
                Command::App(user_id, AppCommand::Focus { game_id })
            }
            ["focus"] => Command::App(user_id, AppCommand::GetFocus),
            ["target", target, ..] => {
                let user_id = W(user_id);
                let Some(game_id) = focus else {
                    return Some(Err(
                        "You have no focused game in which you can target".to_string()
                    ));
                };
                let games = self.games.borrow();
                let Some(game) = games.get(&game_id) else {
                    error!("Could not find focused game {}", game_id);
                    return Some(Err(format!(
                        "Could not find focused game {game_id}! (Bot error!)"
                    )));
                };
                let bytes = target.as_bytes();
                let [c] = bytes[..] else {
                    return Some(Err(format!("Could not parse target {}", target)));
                };
                let Ok(target_id) = game.get_target(c).await else {
                    return Some(Err(format!("Could not find target {}", target)));
                };
                let target = target_id.map(|t| W(UserId(t)));
                Command::Game(game_id, GameCommand::Target { user_id, target })
            }
            ["reveal", ..] => {
                let user_id = W(user_id);
                let Some(game_id) = focus else {
                    return Some(Err(
                        "You have no focused game in which you can reveal".to_string()
                    ));
                };
                Command::Game(game_id, GameCommand::Reveal { user_id })
            }
            _ => return None,
        };
        Some(Ok(cmd))
    }
}

// enum AppRequest {
//     CreateGame { users: Vec<UserId>, rules: Rules, resp: oneshot::Sender<Result<GameId, Error>> },
//     CreateGroup { members: Vec<(UserId, String)>, resp: oneshot::Sender<Result<GroupId, Error>> },
// }

// #[derive(Debug)]
// pub struct LobbyInfo {
//     pub lobby_id: GroupId,
//     pub chat: GroupMeGroup,
//     pub rules: Rules,
// }

// #[derive(Debug, Default)]
// pub struct AppStatus {
//     pub games: HashMap<GameId, GameInfo>,
//     pub groups: HashMap<GroupId, GroupMeGroup>,
//     pub lobbies: HashMap<GroupId, LobbyInfo>,
//     pub focuses: HashMap<UserId, GameId>,
// }

// impl AppStatus {
//     pub fn new() -> Self {
//         Self::default()
//     }
// }
