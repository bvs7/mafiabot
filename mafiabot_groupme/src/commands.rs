use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum Command {
    Lobby(GroupId, LobbyCommand),
    Game(GameId, GameCommand),
    App(UserId, AppCommand),
    Admin(UserId, AdminCommand),
}

#[derive(Debug, Clone)]
pub enum LobbyCommand {
    Start { minutes: u64, min_players: usize },
    Status,
    StatusOf { game_id: GameId },
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
pub enum AppCommand {
    GetFocus,
    Focus { game_id: GameId },
}

#[derive(Debug, Clone)]
pub enum AdminCommand {
    Echo { text: String },
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

pub type Parse<T> = Result<T, String>;
