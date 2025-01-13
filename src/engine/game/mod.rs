mod builder;

use crate::engine::interface::{Election, Event};

use super::{
    interface::{Action, ActionMsg, ActionRx, ActionTx, EventRx, EventTx},
    state::State,
};
use std::{
    sync::{mpsc::RecvError, Arc},
    time::Duration,
};
use tokio::{
    sync::{broadcast, oneshot, watch, Notify, RwLock},
    task::AbortHandle,
    time::{error::Elapsed, sleep},
};
use tracing::{debug, error, info};
use tracing_subscriber::registry::Data;

#[derive(Debug)]
pub enum Quit {
    Continue,
    Save,
    Abort,
}

#[derive(Debug)]
pub struct Game {
    state: Arc<RwLock<State>>,
    action_tx: ActionTx,
    event_tx: EventTx,
    // quit: Arc<watch::Receiver<Quit>>,
}

impl Game {
    async fn new(
        inner: State,
        (action_tx, action_rx): (ActionTx, ActionRx),
        event_tx: EventTx,
    ) -> Arc<Self> {
        let inner = Arc::new(RwLock::new(inner));
        let (game_tx, game_rx) = oneshot::channel::<Arc<Game>>();
        let handle = tokio::spawn(async move {
            let game = game_rx.await.expect("new() should pass game quickly");
            game.run(action_rx);
        })
        .abort_handle();
        let state = Arc::new(Game {
            state: inner,
            action_tx,
            event_tx,
        });
        game_tx.send(state.clone()).expect("Run should not hang up");
        state
    }

    /// Get a broadcast rx subscribed to this game
    fn event_rx(self: &Arc<Self>) -> EventRx {
        self.event_tx.subscribe()
    }
    /// Get a mpsc Sender to send actions to this game
    fn action_tx(self: &Arc<Self>) -> ActionTx {
        self.action_tx.clone()
    }

    #[tracing::instrument]
    async fn run(self: &Arc<Game>, mut action_rx: ActionRx) {
        loop {
            tokio::select! {
                Some(a) = action_rx.recv() => self.handle_action(a).await,
                // () = self.quit.notified() => break,
                else => info!(msg="ActionRx closed"),
            }
        }
    }

    #[tracing::instrument]
    async fn handle_action(self: &Arc<Game>, (action, resp): ActionMsg) {
        debug!(?action);

        // Validate action
        let rstate = self.state.read().await;
        resp.send(rstate.validate_action(&action))
            .unwrap_or_else(|err| error!(?err));
        drop(rstate);
        let mut wstate = self.state.write().await;
        match action {
            Action::Start => {
                let _ = wstate.start(&self.event_tx);
            }
            Action::Vote { voter, ballot } => {
                let result = wstate.vote(voter, ballot, &self.event_tx).unwrap();
                if let Some(Election) = result {
                    // Update current election (if there is one)
                    todo!()
                }
            }
            Action::Reveal { actor } => {
                todo!()
            }
            Action::Scheme { killer, mark } => {
                todo!()
            }
            Action::Target { actor, choice } => {
                todo!()
            }
        };
        todo!()
    }

    async fn election_watcher(self: Arc<Self>, event_rx: EventRx) {}
}
