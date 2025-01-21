use std::{collections::HashMap, hash::Hash, sync::Arc, time::Duration};

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;

/// Run a single mafia game at a time
/*
## Commands

Lobby:
- /start
- /status

Main:
- /vote @player/none

Mafia:
- /target #

DM:
- /target #
- /reveal

The controller will...
- Initialization
    - Start a groupme push subscriber
    - initialize controller state

During operation...
- Listen for commands, and route them
- Listen for events from game and route them


## Things that need to be done:


*/
use crate::engine::{
    interface::{Action, ActionTx, Event, EventRx},
    state::{role::Role, rules::Rules, GameId, PlayerId},
    Game,
};

use super::{api, subscriber::PushWebSocketServer, util::json_access};

async fn update_names(names: &mut HashMap<PlayerId, String>, chat_id: &str) {
    let client = Client::new();
    let group = api::get_group(&client, chat_id).await.unwrap();
    let members: Vec<Value> = json_access(&group, "response.members").unwrap();
    for member in members {
        let name: String = json_access(&member, "nickname").unwrap();
        let id: u64 = json_access(&member, "user_id").unwrap();
        names.insert(id.into(), name);
    }
}

struct Controller {
    lobby_chat_id: String,
    listener: PushWebSocketServer,
    start_msg_id: Option<String>,
    game_context: Option<GameContext>,
    names: HashMap<PlayerId, String>,
}

impl Controller {
    fn new(lobby_chat_id: String) -> Self {
        let mut listener = PushWebSocketServer::new();
        listener.start().expect("Hasn't already started");
        Self {
            lobby_chat_id,
            listener,
            start_msg_id: None,
            game_context: None,
            names: HashMap::new(),
        }
    }

    async fn create_game(this: Arc<Mutex<Self>>, players: Vec<u64>) {
        let main_chat_id = super::MAIN_CHAT_ID.to_string();
        let mafia_chat_id = super::MAFIA_CHAT_ID.to_string();
        let roles: Vec<Role> = todo!();
        let rules = Rules::default();
        let registry = players.iter().copied().zip(roles).collect();
        let game = Game::new(registry, rules);
        let action_tx = game.action_tx();
        let context = GameContext {
            game_id: game.game_id().await,
            action_tx,
            game,
            main_chat_id,
            mafia_chat_id,
            players: Vec::new(),
            names: HashMap::new(),
        };
        let mut ctrl = this.lock().await;
        ctrl.game_context = Some(context);
        ctrl.start_msg_id = None;
        update_names(&mut ctrl.names, super::LOBBY_CHAT_ID).await;
        drop(ctrl);

        let client = Client::new();
        let mut members = Vec::new();
        for player in players {
            let name = ctrl.names.get(&player.into()).unwrap();
            members.push((name.clone(), player.into()));
        }
        api::add_members(&client, &main_chat_id, members)
            .await
            .unwrap();
        tokio::spawn(Self::event_handler(this.clone(), game.event_rx().await));

        tokio::time::sleep(Duration::from_secs(1)).await;
        let h = game.start_action_handler().unwrap();

        let ctrl = this.lock().await;
        let (tx, rx) = tokio::sync::oneshot::channel();
        ctrl.game_context
            .as_ref()
            .unwrap()
            .action_tx
            .send((Action::Start, tx))
            .await
            .unwrap();
        drop(ctrl);
        rx.await.unwrap().unwrap();
    }

    async fn event_handler(this: Arc<Mutex<Self>>, mut event_rx: EventRx) {
        while let Ok(event) = event_rx.recv().await {
            match event {
                Event::Start {
                    id,
                    players,
                    rules,
                    counts,
                } => {
                    // Get game
                    let mut ctrl = this.lock().await;
                    if let Some(ctx) = &mut ctrl.game_context {
                        if ctx.game_id == id {
                            ctx.players = players.iter().map(|(pid, _)| *pid).collect();
                        }
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}

struct GameContext {
    game_id: GameId,
    action_tx: ActionTx,
    game: Game,
    main_chat_id: String,
    mafia_chat_id: String,
    players: Vec<PlayerId>,           // Used to give options
    names: HashMap<PlayerId, String>, // Map of player_id to groupme nickname
}

impl GameContext {}
