use std::collections::HashMap;
use std::env;

use crate::controller::Controller;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::model::id::GuildId;
use serenity::prelude::TypeMapKey;

struct Server {
    controller: HashMap<GuildId, Controller>,
}

impl TypeMapKey for Server {
    type Value = Server;
}

struct Handler;

impl Handler {
    async fn guild_id_create(
        &self,
        ctx: Context,
        guild: serenity::model::id::GuildId,
        _is_new: Option<bool>,
    ) {
        {
            // Add a controller for this guild
            let mut data = ctx.data.write().await;
            let server = data.get_mut::<Server>().unwrap();
            server
                .controller
                .insert(guild, Controller::new(guild, ctx.http.clone()));
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event. This is called whenever a new message is received.
    //
    // Event handlers are dispatched through a threadpool, and so multiple events can be
    // dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            // Sending a message can fail, due to a network error, an authentication error, or lack
            // of permissions to post in the channel, so log to stdout when some error happens,
            // with a description of it.
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a shard is booted, and
    // a READY payload is sent by Discord. This payload contains data like the current user's guild
    // Ids, current user data, private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        for guild in ready.guilds {
            println!("{:?} is in the guild list", guild);
            if guild.unavailable {
                println!("Guild {} is unavailable...", guild.id);
                // let result = create_mafia_command(guild.id, &ctx.http).await;
                // if let Err(why) = result {
                //     println!("Error creating mafia command: {why:?}");
                // }
            }
            self.guild_id_create(ctx.clone(), guild.id, None).await;
        }
    }

    async fn guild_create(
        &self,
        ctx: Context,
        guild: serenity::model::guild::Guild,
        _is_new: Option<bool>,
    ) {
        self.guild_id_create(ctx, guild.id, None).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let guild_id: Option<GuildId> = match &interaction {
            Interaction::Command(command) => command.guild_id.clone(),
            Interaction::Component(component) => component.guild_id.clone(),
            _ => None,
        };
        if let Some(guild_id) = guild_id {
            let ctx2 = ctx.clone();
            let mut data = ctx2.data.write().await;
            let server = data.get_mut::<Server>().unwrap();
            if let Some(controller) = server.controller.get_mut(&guild_id) {
                controller
                    .interaction_create(ctx.clone(), interaction)
                    .await;
            }
        } else {
            println!("No guild id found for interaction: {:#?}", interaction);
        }
    }
}

fn get_gateway_intents() -> GatewayIntents {
    GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
}

pub async fn start() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = get_gateway_intents();

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Grab the lock for the client data and insert the new Server struct
    {
        let mut data = client.data.write().await;
        data.insert::<Server>(Server {
            controller: HashMap::new(),
        });
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
