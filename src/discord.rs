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

pub enum Access {
    None,
    View,
    Message,
}

pub type DiscordError = ();

pub fn send_to_channel(channel: ChannelID, msg: String) -> Result<MessageID, DiscordError> {
    println!("send_to_channel({}, {})", channel, msg);
    todo!();
}

pub fn send_to_thread(
    channel: ChannelID,
    user: UserID,
    msg: String,
) -> Result<MessageID, DiscordError> {
    println!("send_to_thread({}, {}, {})", channel, user, msg);
    todo!();
}

pub fn send_target_message(
    channel: ChannelID,
    actor: UserID,
    options: &Vec<UserID>,
    verb: &str,
) -> Result<(), DiscordError> {
    todo!();
}

pub fn send_mark_message(channel: ChannelID, options: &Vec<UserID>) -> Result<(), DiscordError> {
    todo!();
}

pub fn get_name(user: UserID) -> Result<String, DiscordError> {
    todo!();
}

pub fn get_lobby_channels(guild: GuildID) -> Result<(ChannelID, ChannelID), DiscordError> {
    // Lobby should be the first channel, named "lobby", in a category "mafiabot"
    // If it doesn't exist, create it.
    todo!();
}

pub fn create_game_channels(category: ChannelID) -> Result<GameChannels, DiscordError> {
    // Create a main channel
    // Create a mafia channel

    // Create a join button in lobby channel
    // Create a leave button in main channel
    todo!();
}

pub fn create_channel(guild: GuildID, name: String) -> Result<ChannelID, DiscordError> {
    println!("create_channel({})", name);

    todo!();
}

pub fn delete_game_channels(channels: GameChannels) -> Result<(), DiscordError> {
    todo!();
}

pub fn get_users_in_channel(channel: ChannelID) -> Result<Vec<UserID>, DiscordError> {
    todo!();
}

pub fn add_users_to_channel(channel: ChannelID, user: Vec<UserID>) -> Result<(), DiscordError> {
    println!("add_users_to_channel({}, {:?})", channel, user);

    todo!();
}

pub fn remove_users_from_channel(
    channel: ChannelID,
    users: Vec<UserID>,
) -> Result<(), DiscordError> {
    todo!();
}

pub fn get_reacts_to_message(
    channel: ChannelID,
    message: MessageID,
    emoji: String,
) -> Result<Vec<UserID>, DiscordError> {
    println!("get_reacts_to_message({}, {}, {})", channel, message, emoji);
    todo!();
}

pub fn change_permission(
    channel: ChannelID,
    user: UserID,
    access: Access,
) -> Result<(), DiscordError> {
    todo!();
}

pub fn change_channel_permission(channel: ChannelID, access: Access) -> Result<(), DiscordError> {
    todo!();
}
