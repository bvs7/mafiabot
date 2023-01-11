use crate::{
    core::{Action, Choice},
    discord::{ChannelID, MessageID, UserID},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Lobby(LobbyCommand),
    Game(Action<UserID>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LobbyCommand {
    Init, // Done in Lobby
    Join(UserID),
    Leave(UserID),
    Start,
    Close,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GameCommand {
    Vote {
        voter: UserID,
        ballot: Option<Choice<UserID>>,
    },
    Reveal {
        celeb: UserID,
    },
    Target {
        actor: UserID,
        target: Choice<UserID>,
    },
    Mark {
        killer: UserID,
        mark: Choice<UserID>,
    },
}
