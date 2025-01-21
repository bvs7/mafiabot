use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock, TryLockError};

pub mod interface;
pub mod rolegen;
pub mod state;

use interface::*;
use rolegen::*;
use state::*;
use state::{role::*, rules::*};
use tokio::task::JoinHandle;

pub struct Game {
    state: Arc<RwLock<State>>, // Arc for access via different places

    action_tx: ActionTx, // Allow cloning
    action_rx: Arc<Mutex<ActionRx>>, // Option so we can take it
                         // event_tx: EventTx,               // Allow subscribing
}

impl Game {
    // How can state be created without roles? Should rolegen be rolled in?
    pub fn new(registry: Vec<(u64, Role)>, rules: Rules) -> Self {
        let game_id = todo!("Get game_id from a file?");
        let state = State::new(game_id, registry, rules);
        Self::from_state(state)
    }

    pub fn from_state(state: State) -> Self {
        let state = Arc::new(RwLock::new(state));
        let (action_tx, action_rx) = mpsc::channel(100);
        let action_rx = Arc::new(Mutex::new(action_rx));
        Self {
            state,
            action_tx,
            action_rx,
        }
    }

    pub async fn game_id(&self) -> GameId {
        let rstate = self.state.read().await;
        rstate.game_id()
    }

    pub fn action_tx(&self) -> ActionTx {
        self.action_tx.clone()
    }
    pub async fn event_rx(&self) -> EventRx {
        let rstate = self.state.read().await;
        rstate.subscribe()
    }

    /// Spawn a thread to start handling actions from the action_rx queue
    pub fn start_action_handler(&self) -> Result<JoinHandle<()>, TryLockError> {
        let rx = self.action_rx.clone().try_lock_owned()?;
        let state = self.state.clone();
        let h = tokio::spawn(async move {
            let mut rx = rx;
            State::action_handler(state, &mut rx).await
        });
        Ok(h)
    }

    /// Use current task to start handling actions from the action_rx queue
    pub async fn run_action_handler(&self) -> Result<(), TryLockError> {
        let mut rx = self.action_rx.clone().try_lock_owned()?;
        State::action_handler(self.state.clone(), &mut rx).await;
        Ok(())
    }
}

// struct GameBuilder {
//     state: StateBuilder,
// }
