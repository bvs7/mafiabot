use std::env;

use serenity::async_trait;
use serenity::client::bridge::gateway::event::ShardStageUpdateEvent;
use serenity::json::Value;
use serenity::model::application::interaction::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::collections::HashMap;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    /// Dispatched when the cache has received and inserted all data from
    /// guilds.
    ///
    /// This process happens upon starting your bot and should be fairly quick.
    /// However, cache actions performed prior this event may fail as the data
    /// could be not inserted yet.
    ///
    /// Provides the cached guilds' ids.
    #[cfg(feature = "cache")]
    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        todo!("Add a controller for each Guild");
    }

    /// Dispatched when a channel is updated.
    ///
    /// Provides the old channel data, and the new data.
    #[cfg(feature = "cache")]
    async fn channel_update(&self, _ctx: Context, _old: Option<Channel>, _new: Channel) {
        todo!("Check name and topic?");
    }

    /// Dispatched when a user is banned from a guild.
    ///
    /// Provides the guild's id and the banned user's data.
    async fn guild_ban_addition(&self, _ctx: Context, _guild_id: GuildId, _banned_user: User) {
        todo!("React to this?");
    }

    /// Dispatched when a user's ban is lifted from a guild.
    ///
    /// Provides the guild's id and the lifted user's data.
    async fn guild_ban_removal(&self, _ctx: Context, _guild_id: GuildId, _unbanned_user: User) {
        todo!("React to this?");
    }

    /// Dispatched when a guild is deleted.
    ///
    /// Provides the partial data of the guild sent by discord,
    /// and the full data from the cache, if available.
    ///
    /// The [`unavailable`] flag in the partial data determines the status of the guild.
    /// If the flag is false, the bot was removed from the guild, either by being
    /// kicked or banned. If the flag is true, the guild went offline.
    ///
    /// [`unavailable`]: UnavailableGuild::unavailable
    #[cfg(feature = "cache")]
    async fn guild_delete(
        &self,
        _ctx: Context,
        _incomplete: UnavailableGuild,
        _full: Option<Guild>,
    ) {
        todo!("Remove the controller for this guild");
    }

    /// Dispatched when a user joins a guild.
    ///
    /// Provides the guild's id and the user's member data.
    ///
    /// Note: This event will not trigger unless the "guild members" privileged intent
    /// is enabled on the bot application page.
    async fn guild_member_addition(&self, _ctx: Context, _new_member: Member) {
        todo!("React to this?");
    }

    /// Dispatched when a user's membership ends by leaving, getting kicked, or being banned.
    ///
    /// Provides the guild's id, the user's data, and the user's member data if available.
    ///
    /// Note: This event will not trigger unless the "guild members" privileged intent
    /// is enabled on the bot application page.
    #[cfg(feature = "cache")]
    async fn guild_member_removal(
        &self,
        _ctx: Context,
        _guild_id: GuildId,
        _user: User,
        _member_data_if_available: Option<Member>,
    ) {
        todo!("React to this?");
    }

    /// Dispatched when a member is updated (e.g their nickname is updated).
    ///
    /// Provides the member's old data (if available) and the new data.
    ///
    /// Note: This event will not trigger unless the "guild members" privileged intent
    /// is enabled on the bot application page.
    #[cfg(feature = "cache")]
    async fn guild_member_update(
        &self,
        _ctx: Context,
        _old_if_available: Option<Member>,
        _new: Member,
    ) {
    }

    /// Dispatched when the data for offline members was requested.
    ///
    /// Provides the guild's id and the data.
    async fn guild_members_chunk(&self, _ctx: Context, _chunk: GuildMembersChunkEvent) {}

    /// Dispatched when a role is updated.
    ///
    /// Provides the guild's id, the role's old (if available) and new data.
    #[cfg(feature = "cache")]
    async fn guild_role_update(
        &self,
        _ctx: Context,
        _old_data_if_available: Option<Role>,
        _new: Role,
    ) {
    }

    /// Dispatched when a guild became unavailable.
    ///
    /// Provides the guild's id.
    async fn guild_unavailable(&self, _ctx: Context, _guild_id: GuildId) {
        todo!("Remove the controller for this guild?");
    }

    /// Dispatched when the guild is updated.
    ///
    /// Provides the guild's old full data (if available) and the new, albeit partial data.
    #[cfg(feature = "cache")]
    async fn guild_update(
        &self,
        _ctx: Context,
        _old_data_if_available: Option<Guild>,
        _new_but_incomplete: PartialGuild,
    ) {
    }

    /// Dispatched when a message is created.
    ///
    /// Provides the message's data.
    async fn message(&self, _ctx: Context, _new_message: Message) {
        todo!("Handle messages?");
    }

    /// Dispatched when a new reaction is attached to a message.
    ///
    /// Provides the reaction's data.
    async fn reaction_add(&self, _ctx: Context, _add_reaction: Reaction) {
        todo!("Handle reactions");
    }

    /// Dispatched when a reaction is detached from a message.
    ///
    /// Provides the reaction's data.
    async fn reaction_remove(&self, _ctx: Context, _removed_reaction: Reaction) {
        todo!("Handle reactions");
    }

    /// Dispatched when all reactions of a message are detached from a message.
    ///
    /// Provides the channel's id and the message's id.
    async fn reaction_remove_all(
        &self,
        _ctx: Context,
        _channel_id: ChannelId,
        _removed_from_message_id: MessageId,
    ) {
    }

    /// Dispatched upon startup.
    ///
    /// Provides data about the bot and the guilds it's in.
    async fn ready(&self, _ctx: Context, _data_about_bot: Ready) {
        todo!("Startup");
    }

    /// Dispatched upon reconnection.
    async fn resume(&self, _ctx: Context, _: ResumedEvent) {}

    /// Dispatched when a shard's connection stage is updated
    ///
    /// Provides the context of the shard and the event information about the update.
    async fn shard_stage_update(&self, _ctx: Context, _: ShardStageUpdateEvent) {}

    /// Dispatched when an unknown event was sent from discord.
    ///
    /// Provides the event's name and its unparsed data.
    async fn unknown(&self, _ctx: Context, _name: String, _raw: Value) {}

    /// Dispatched when an interaction is created (e.g a slash command was used or a button was clicked).
    ///
    /// Provides the created interaction.
    async fn interaction_create(&self, _ctx: Context, _interaction: Interaction) {}

    /// Dispatched when a guild integration is created.
    ///
    /// Provides the created integration.
    async fn integration_create(&self, _ctx: Context, _integration: Integration) {}

    /// Dispatched when a guild integration is updated.
    ///
    /// Provides the updated integration.
    async fn integration_update(&self, _ctx: Context, _integration: Integration) {}

    /// Dispatched when a guild integration is deleted.
    ///
    /// Provides the integration's id, the id of the guild it belongs to, and its associated application id
    async fn integration_delete(
        &self,
        _ctx: Context,
        _integration_id: IntegrationId,
        _guild_id: GuildId,
        _application_id: Option<ApplicationId>,
    ) {
    }

    /// Dispatched when a thread is updated.
    ///
    /// Provides the updated thread.
    async fn thread_update(&self, _ctx: Context, _thread: GuildChannel) {}

    /// Dispatched when a thread is deleted.
    ///
    /// Provides the partial deleted thread.
    async fn thread_delete(&self, _ctx: Context, _thread: PartialGuildChannel) {}

    /// Dispatched when the current user gains access to a channel
    ///
    /// Provides the threads the current user can access, the thread members,
    /// the guild Id, and the channel Ids of the parent channels being synced.
    async fn thread_list_sync(&self, _ctx: Context, _thread_list_sync: ThreadListSyncEvent) {}

    /// Dispatched when the [`ThreadMember`] for the current user is updated.
    ///
    /// Provides the updated thread member.
    async fn thread_member_update(&self, _ctx: Context, _thread_member: ThreadMember) {}

    /// Dispatched when anyone is added to or removed from a thread. If the current user does not have the [`GatewayIntents::GUILDS`],
    /// then this event will only be sent if the current user was added to or removed from the thread.
    ///
    /// Provides the added/removed members, the approximate member count of members in the thread,
    /// the thread Id and its guild Id.
    ///
    /// [`GatewayIntents::GUILDS`]: crate::model::gateway::GatewayIntents::GUILDS
    async fn thread_members_update(
        &self,
        _ctx: Context,
        _thread_members_update: ThreadMembersUpdateEvent,
    ) {
    }
}
