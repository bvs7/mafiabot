use std::{env, future::Future, path::PathBuf, sync::Arc};

use tokio::{
    sync::{watch, RwLock},
    time::error::Elapsed,
};

use crate::prelude::*;

use super::action;

#[derive(thiserror::Error, Debug)]
enum GameIdError {
    #[error("Failed to read game_id file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse game_id file: {0}")]
    ParseError(#[from] std::num::ParseIntError),
    #[error("Failed to write game_id file")]
    WriteError,
}

#[derive(Debug, Clone, Copy, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct GameId(u64);

impl GameId {
    const DEFAULT_DIR: &'static str = "mafia";

    pub fn new() -> Result<Self, GameIdError> {
        // First try env variable to get mafia directory
        let dir = match env::var("MAFIA_DIR").map(PathBuf::from) {
            Ok(dir) => dir,
            Err(err) => {
                warn!("Failed to get MAFIA_DIR: {}", err);
                PathBuf::from(Self::DEFAULT_DIR)
            }
        };
        let fname = dir.join("game_id");
        let id: u64 = std::fs::read_to_string(&fname)?.parse()?;
        // Write the file with id + 1
        std::fs::write(&fname, (id + 1).to_string()).unwrap_or_else(|_| {
            error!("Failed to write game_id file");
        });

        Ok(Self(id))
    }
}

impl From<GameId> for u64 {
    fn from(value: GameId) -> Self {
        value.0
    }
}
impl From<u64> for GameId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for GameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
/*
If someone wants to make a new game... They pass in Rules and a list of userIds (which can Into<Pid>).
Then rolegen is determined from rules, roles are generated, and the state is created. Alternatively, the
state could be loaded from a save file.

Maybe this is where we want traits?

We want... an event listener, an action receiver, and a state updater. In loops

So let's think about how to do this. Definitely some kind of initialization and closing.

let's write out the loops then try writing a basic handler

*/

pub trait ActionHandler<P> {
    fn init(&mut self, game: &Game);
    async fn recv_action(&mut self) -> Option<Action<P>>;
    async fn resp_action(&mut self, result: Result<(), Error>);
    async fn update_status(&mut self, state: &State);
}

pub trait EventHandler {
    fn init(&mut self, game: &Game);
    fn handle_event(&mut self, event: Event) -> impl Future<Output = ()> + Send;
}

// Wrapper for the game state, which takes takes an event_tx (and an action_rx?)
// Then has an async run method that listens for actions and updates the state...
// Or should this just be done one level up?
// Should ActionHandler and EventHandler be separate? Or should they be combined?

/*
If they are combined, then they need to be able to share data between two threads...
It seems like it would be too restrictive to not let event handler get &mut self...

What does ActionHandler need?
- It needs to receive actions, easy, just have a channel
- It needs to send action responses, easy, just have a channel
- It needs to update the status, and for that it needs names? Should names be part of status? No.
Names are a part of the GroupMe Group objects...

Maybe EventHandler can wrap some shared state with Action Handler?

It does seem like they need to be separate to allow passing the event handler to another thread...

What does the eventhandler need.
- It needs to receive events from the state, use the event channel
- It needs to send out dms and group messages. It needs access to GroupMeGroups and some kind of Pid -> UserIds
    What could that be? It's probably shared, and we want to be able to update it.
    Who updates it? Probably the top level command input point. So... It could be a RwLock of GroupMeGroups?
*/

pub struct Game {
    id: GameId,
    state: State,
}

impl Game {
    pub async fn create<P, E, A>(
        players: impl IntoIterator<Item = impl Into<Pid>>,
        rules: Rules,
        mut action_handler: A,
        mut event_handler: E,
    ) where
        P: Into<Pid> + Copy,
        A: ActionHandler<P>,
        E: EventHandler + Send + 'static,
    {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let roles: Vec<Role> = Vec::new(); // Generate roles from rules!
        let registry = players.into_iter().zip(roles).collect::<Vec<_>>();
        let mut state = State::new(registry, rules);
        state.event_tx = Some(event_tx);

        let game = Self { id: GameId::new().unwrap_or_default(), state };

        action_handler.init(&game);
        event_handler.init(&game);

        tokio::spawn(Game::event_handler(event_rx, event_handler));

        game.action_handler(action_handler).await;
    }

    #[instrument(skip_all)]
    pub async fn event_handler(
        mut event_rx: mpsc::UnboundedReceiver<Event>,
        mut handler: impl EventHandler,
    ) {
        loop {
            match event_rx.recv().await {
                Some(event) => handler.handle_event(event).await,
                None => {
                    info!("Event channel closed");
                    break;
                }
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn action_handler<P, A>(mut self, mut handler: A)
    where
        P: Into<Pid> + Copy,
        A: ActionHandler<P>,
    {
        loop {
            let timeout = self.state.update();
            handler.update_status(&self.state).await;
            let dur = match timeout.map(|t| (t - Local::now()).to_std()) {
                Some(Ok(dur)) => dur,           // Wait for timeout
                Some(Err(e)) => Duration::ZERO, // Time already lapsed
                None => Duration::MAX,          // No timeout to wait for
            };

            let action = match tokio::time::timeout(dur, handler.recv_action()).await {
                Err(Elapsed { .. }) => continue,
                Ok(None) => break,
                Ok(Some(action)) => action,
            };

            let result = self.state.validate_action(action);
            match result {
                Err(err) => handler.resp_action(Err(err)).await,
                Ok(action) => {
                    handler.resp_action(Ok(()));
                    self.state.perform_action(action);
                }
            }
        }
    }
}

type RespContext = oneshot::Sender<Result<(), Error>>;

struct Statuses {
    games: HashMap<GameId, Status>,
    lobbies: HashMap<u64, HashMap<u64, String>>,
}

struct GroupMeGroup {
    id: u64,
    names: HashMap<u64, String>,
}

struct BasicActionHandler {
    game_id: GameId,
    app_comms: Arc<RwLock<AppComms>>,
    action_rx: mpsc::Receiver<(Action<u64>, RespContext)>,
    resp: Option<RespContext>,
    status_tx: watch::Sender<Statuses>,
}

impl ActionHandler<u64> for BasicActionHandler {
    fn init(&mut self, game: &Game) {
        // let (action_tx, action_rx) = mpsc::channel(100);
        // game.action_tx = Some(action_tx);
        // self.action_rx = action_rx;
    }

    async fn recv_action(&mut self) -> Option<Action<u64>> {
        self.action_rx.recv().await.map(|(action, ctx)| {
            self.resp = Some(ctx);
            action
        })
    }

    async fn resp_action(&mut self, result: Result<(), Error>) {
        if let Some(ctx) = self.resp.take() {
            let _ = ctx.send(result);
        }
    }

    async fn update_status(&mut self, state: &State) {
        let names = self.app_comms.read().await.groups.get(&self.game_id).unwrap().names.clone();
        let status = state.status();
    }
}

struct BasicEventHandler {
    game_id: GameId,
    app_comms: Arc<RwLock<AppComms>>,
    event_rx: mpsc::UnboundedSender<Event>,
}

impl EventHandler for BasicEventHandler {
    fn init(&mut self, game: &Game) {
        // let (event_tx, event_rx) = mpsc::unbounded_channel();
        // game.event_tx = Some(event_tx);
        // self.event_rx = event_rx;
    }

    fn handle_event(&mut self, event: Event) -> impl Future<Output = ()> + Send {
        self.event_rx.send(event)
    }
}

struct AppComms {
    groups: HashMap<u64, GroupMeGroup>, // GroupId -> GroupMeGroup
    users: HashMap<u64, u64>,           // Pid -> UserId
}
