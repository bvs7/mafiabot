mod builder;

use crate::engine::{
    interface::{Election, Event, Vote},
    state::Choice,
};

use super::{
    interface::{Action, ActionMsg, ActionRx, ActionTx, EventRx, EventTx},
    state::State,
    timer::{TimeTx, Timer},
};
use chrono::Local;
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast::error::RecvError, Mutex, Notify, RwLock, TryLockError};
use tracing::{debug, error, info};

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
    quit: Notify,
    run_lock: Mutex<()>,
}

impl Game {
    fn new(
        state: State,
        (action_tx, action_rx): (ActionTx, ActionRx),
        event_tx: EventTx,
    ) -> Arc<Self> {
        let state = Arc::new(RwLock::new(state));
        let quit = Notify::new();
        let run_lock = Mutex::new(());
        let game = Arc::new(Self {
            state,
            action_tx,
            event_tx,
            quit,
            run_lock,
        });
        let s = game.clone();
        tokio::spawn(s.run(action_rx));
        game
    }

    /// Get a broadcast rx subscribed to this game
    pub fn event_rx(self: &Arc<Self>) -> EventRx {
        self.event_tx.subscribe()
    }
    /// Get a mpsc Sender to send actions to this game
    pub fn action_tx(self: &Arc<Self>) -> ActionTx {
        self.action_tx.clone()
    }

    pub fn quit(self: Arc<Self>) {
        self.quit.notify_one()
    }

    /// Synchronous run...
    #[tracing::instrument]
    async fn run(self: Arc<Self>, action_rx: ActionRx) -> Result<(), TryLockError> {
        let s = self.clone();
        let action_handler = tokio::spawn(s.action_handler(action_rx));
        let s = self.clone();
        let event_rx = self.event_rx();
        let election_watcher = tokio::spawn(s.election_watcher(event_rx));
        let s = self.clone();
        let event_rx = self.event_rx();
        let dawn_watcher = tokio::spawn(s.dawn_watcher(event_rx));

        self.quit.notified().await;

        action_handler.abort();
        election_watcher.abort();
        dawn_watcher.abort();

        Ok(())
    }

    #[tracing::instrument]
    async fn action_handler(self: Arc<Self>, mut action_rx: ActionRx) {
        loop {
            let input = action_rx.recv().await;
            let Some((action, responder)) = input else {
                info!(msg = "ActionRx closed");
                break;
            };

            let rstate = self.state.read().await;
            let resp = rstate.validate_action(&action);
            drop(rstate);
            let _ = responder.send(resp.clone()).inspect_err(|e| error!(?e));
            if let Err(e) = &resp {
                info!(msg = "Invalid action recv'd", ?action, ?e);
            } else {
                let mut wstate = self.state.write().await;
                let tx = &self.event_tx;
                let result = match action {
                    Action::Start => wstate.start(tx),
                    Action::Vote { voter, ballot } => wstate.vote(voter, ballot, tx),
                    Action::Reveal { actor } => wstate.reveal(actor, tx),

                    Action::Scheme { killer, mark } => wstate.scheme(killer, mark, tx),
                    Action::Target { actor, choice } => wstate.target(actor, choice, tx),
                };
            }
        }
    }

    fn election_timer(self: Arc<Self>, timer: Timer, choice: Choice) {
        tokio::spawn(async move {
            if timer.await {
                let mut wstate = self.state.write().await;
                wstate.try_election(choice, &self.event_tx);
                drop(wstate);
            }
        });
    }

    #[tracing::instrument]
    async fn election_watcher(self: Arc<Self>, mut event_rx: EventRx) {
        let mut election: Option<(Choice, TimeTx)> = None;
        loop {
            match event_rx.recv().await {
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => {
                    error!(msg = "Missed recv events", ?n)
                }
                Ok(Event::Vote {
                    voter,
                    ballot,
                    former,
                }) => debug!(?voter, ?ballot, ?former),
                Ok(Event::Election(_)) => {
                    election = None;
                    continue;
                }
                _ => continue,
            };

            let rstate = self.state.read().await;
            let new_election = rstate.check_election();
            drop(rstate);
            // If new, and new isn't old, then stop timer and start new timer

            // test for cancelling old election
            let cancel_tx = match (&new_election, &election) {
                (None, Some((_, tx))) => Some(tx),
                (Some(new), Some((old, tx))) if new != old => Some(tx),
                _ => None,
            };
            if let Some(tx) = cancel_tx {
                let _ = tx.send(None); // Cancel timer
                election = None;
            }
            if let Some(choice) = new_election {
                let soon = Local::now() + Duration::from_secs(10);
                let (timer, tx) = Timer::new(soon);
                election = Some((choice.clone(), tx));
                self.clone().election_timer(timer, choice);
            }
        }
    }

    fn dawn_timer(self: Arc<Self>, timer: Timer) {
        tokio::spawn(async move {
            if timer.await {
                let mut wstate = self.state.write().await;
                wstate.dawn(&self.event_tx);
                drop(wstate);
            }
        });
    }

    #[tracing::instrument]
    async fn dawn_watcher(self: Arc<Self>, mut event_rx: EventRx) {
        let mut dawn_imminent: bool = false;
        loop {
            match event_rx.recv().await {
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => {
                    error!(msg = "Missed recv events", ?n);
                }
                Ok(Event::Target { .. }) | Ok(Event::Scheme { .. }) => {}
                _ => continue,
            }

            let rstate = self.state.read().await;
            let dawn = rstate.check_dawn();

            if !dawn_imminent && dawn {
                dawn_imminent = true;
                let soon = Local::now() + Duration::from_secs(10);
                let (timer, _) = Timer::new(soon);
                self.clone().dawn_timer(timer);
            }
        }
    }
}
