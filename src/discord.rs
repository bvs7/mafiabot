// Discord interface for mafiabot

// Must implement a server that parses Discord API callback HTTP requests
// Routes those requests into Commands for the Mafiabot controller.

// Also implements a way to:
// - send messages to channels,
// - create channels,
// - interface with channels

use crate::controller::GameChannels;

use super::core::RawPID;

pub type UserID = u64;
impl RawPID for UserID {}

pub type ChannelID = u64;
pub type MessageID = u64;
pub type GuildID = u64;

mod parser;
mod services;

pub use services::*;
