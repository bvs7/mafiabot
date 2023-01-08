use crate::discord::{ChannelID, MessageID, UserID};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Command {
    pub sender: UserID,
    pub channel: ChannelID,
    pub kind: CommandKind,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommandKind {
    Init,
    Start,
    Vote { text: String, mentions: Vec<UserID> },
    Target { text: String },
    Mark { text: String },
    Reveal,
    Status,
}
