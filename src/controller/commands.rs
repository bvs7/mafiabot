use crate::{
    core::Choice,
    discord::{ChannelID, MessageID, UserID},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    Init, // Done in Lobby
    Join(UserID),
    Leave(UserID),
    Start,
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
