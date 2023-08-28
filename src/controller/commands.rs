use crate::{
    core::{Action, Choice, PID},
    discord::{ChannelID, MessageID}
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Lobby(LobbyCommand),
    Game(Action),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LobbyCommand {
    Init, // Done in Lobby
    Join(PID),
    Leave(PID),
    Start,
    Close,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GameCommand {
    Vote {
        voter: PID,
        ballot: Option<Choice>,
    },
    Reveal {
        celeb: PID,
    },
    Target {
        actor: PID,
        target: Choice,
    },
    Mark {
        killer: PID,
        mark: Choice,
    },
}
