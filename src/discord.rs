// Discord interface for mafiabot

// Must implement a server that parses Discord API callback HTTP requests
// Routes those requests into Commands for the Mafiabot controller.

// Also implements a way to:
// - send messages to channels,
// - create channels,
// - interface with channels

use super::core::RawPID;

pub type UserID = u64;
impl RawPID for UserID {}

pub type ChannelID = u64;
pub type MessageID = u64;
pub type GuildID = u64;

pub enum Access {
    None,
    View,
    Message,
}

pub fn send_to_channel(channel: ChannelID, msg: String) -> MessageID {
    println!("send_to_channel({}, {})", channel, msg);

    todo!();
}

pub fn create_channel(guild: GuildID, name: String) -> ChannelID {
    println!("create_channel({})", name);

    todo!();
}

pub fn create_single_user_channel(user: UserID) -> ChannelID {
    todo!();
}

pub fn add_users_to_channel(channel: ChannelID, user: Vec<UserID>) -> Result<(), ()> {
    println!("add_users_to_channel({}, {:?})", channel, user);

    todo!();
}

pub fn get_reacts_to_message(channel: ChannelID, message: MessageID, emoji: String) -> Vec<UserID> {
    println!("get_reacts_to_message({}, {}, {})", channel, message, emoji);
    todo!();
}

pub fn change_permission(channel: ChannelID, user: UserID, access: Access) {
    todo!();
}

/// Get a list of all channel IDs this bot is a part of
pub fn get_all_channels() -> Vec<ChannelID> {
    todo!();
}
