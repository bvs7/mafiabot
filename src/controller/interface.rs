use crate::discord_::types::{ChannelID, UserID};

pub struct Command {
    sender: UserID,
    channel: ChannelID,
    kind: CommandKind,
}
pub enum CommandKind {
    Init,
    Start,
    Vote { text: String, mentions: Vec<UserID> },
    Target { text: String },
    Mark { text: String },
    Reveal,
    Status,
}
