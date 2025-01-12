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

    // async fn election_watcher(self: &Arc<Game>) {
    //     let mut event_rx = self.event_rx();
    //     // Initialize state

    //     struct DayState {
    //         thresh: usize,
    //         pthresh: usize,
    //         elect: Option<(Election, AbortHandle)>,
    //     }

    //     let mut ds: Option<DayState> = None; // get this from inner state
    //     loop {
    //         use broadcast::error::RecvError;
    //         match (&mut ds, event_rx.recv().await) {
    //             (None, Ok(Event::Day { counts, .. })) => {
    //                 let n = counts.len();
    //                 ds = Some(DayState {
    //                     thresh: (n / 2) + 1,
    //                     pthresh: (n + 1) / 2,
    //                     elect: None,
    //                 });
    //             }
    //             (
    //                 // TODO: no wait we need to figure out these condidtions
    //                 Some(DayState {
    //                     thresh,
    //                     pthresh,
    //                     elect,
    //                 }),
    //                 Ok(Event::Vote {
    //                     voter,
    //                     ballot: Some((choice, n)),
    //                     ..
    //                 }),
    //             ) if elect.as_ref().is_none_or(|(e, _)| e.choice != choice) => {
    //                 // Check if threshold is met
    //                 let t = if choice.is_some() { thresh } else { pthresh };
    //                 if n >= *t {
    //                     let rstate = self.state.read().await;
    //                     let e = rstate.get_election(choice, *t, voter);
    //                     drop(rstate);
    //                     let s = self.clone();
    //                     if let Some(el) = e {
    //                         let el1 = el.clone();
    //                         let h = tokio::spawn(async move { s.schedule_election(el1).await })
    //                             .abort_handle();
    //                         *elect = Some((el, h));
    //                     }
    //                 }
    //             }
    //             (
    //                 Some(DayState {
    //                     thresh,
    //                     pthresh,
    //                     elect,
    //                 }),
    //                 Ok(Event::Vote {
    //                     voter,
    //                     former: Some((choice, n)),
    //                     ..
    //                 }),
    //             ) if elect.as_ref().is_some_and(|(e, _)| e.choice == choice) => {}

    //             (_, Err(RecvError::Closed)) => {}
    //             (_, Err(RecvError::Lagged(n))) => {}
    //             (_) => {}
    //         }
    //     }
    // }

    // async fn schedule_election(self: &Arc<Game>, elect: Election) {
    //     tokio::time::sleep(Duration::from_secs(10)).await;
    //     let mut wstate = self.state.write().await;
    //     wstate
    //         .election(elect, &self.event_tx)
    //         .unwrap_or_else(|err| error!(?err));
    // }

    async fn spawn_election_timer(self: Arc<Self>) -> WatchTx {
        let (tx, rx) = watch::channel(None);
        let s = self.clone();
        tokio::spawn(async move {
            let mut rx = rx;
            loop {
                if !timer(&mut rx).await {
                    // Channel closed. quit
                    break;
                }
                // timer returned, election must occur
                let mut wstate = s.state.write().await;
                wstate.election(&self.event_tx).unwrap();
                drop(wstate);
            }
        });
        tx
    }

    // async fn elect(self: &Arc<Game>, choice: Option<u64>, hammer: u64) {
    //     tokio::time::sleep(Duration::from_secs(10)).await;
    //     let mut write_inner = self.state.write().await;
    //     if let Err(e) = write_inner.election(choice, hammer, &self.tx).await {
    //         error!(?e);
    //     }
    //     drop(write_inner);
    // }
}

use tokio::time::timeout;

async fn timer(rx: &mut WatchRx) -> bool {
    let mut t = rx.borrow_and_update().clone();
    loop {
        let r = match t.map(|time| (time - Local::now()).to_std().ok()) {
            Some(Some(dt)) => timeout(dt, rx.changed()).await.ok(), // Some(Some(dt)) is forward offset.
            Some(None) => None,                                     // Some(None) is immediate.
            None => Some(rx.changed().await),                       // None is no timer
        };
        match r {
            None => return true,          // Time has elapsed
            Some(Ok(_)) => {}             // changed, continue
            Some(Err(_)) => return false, // Channel dropped
        }
        t = rx.borrow_and_update().clone();
    }
}

#[cfg(test)]
mod test {
    use chrono::Local;
    use tokio::{
        sync::{watch, Mutex},
        time::sleep,
    };

    use super::{timer, WatchTx};
    use std::{sync::Arc, time::Duration};

    async fn setup() -> (WatchTx, Arc<Mutex<i32>>) {
        let (tx, mut rx) = watch::channel(None);
        let m = Arc::new(Mutex::new(0));
        let m2 = m.clone();
        let h = tokio::spawn(async move {
            if timer(&mut rx).await {
                let mut mm = m2.lock().await;
                *mm = 1;
                drop(mm);
            } else {
                let mut mm = m2.lock().await;
                *mm = 2;
                drop(mm);
            }
        });
        (tx, m)
    }

    #[tokio::test]
    async fn watch_basic() {
        let (tx, m) = setup().await;

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        let _ = tx.send(Some(Local::now()));

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 1);
        drop(mm);
    }

    #[tokio::test]
    async fn watch_drop() {
        let (tx, m) = setup().await;

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        drop(tx);

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 2);
        drop(mm);
    }

    #[tokio::test]
    async fn watch_wait() {
        let (tx, m) = setup().await;

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        let _ = tx.send(Some(Local::now() + Duration::from_secs(3)));

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        sleep(Duration::from_secs(4)).await;

        let mm = m.lock().await;
        assert!(*mm == 1);
        drop(mm);
    }

    #[tokio::test]
    async fn watch_wait_wait() {
        let (tx, m) = setup().await;

        sleep(Duration::from_millis(50)).await;

        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        for i in 0..5 {
            let _ = tx.send(Some(Local::now() + Duration::from_secs(1)));

            sleep(Duration::from_millis(500)).await;

            let mm = m.lock().await;
            assert!(*mm == 0);
            drop(mm);
        }

        sleep(Duration::from_secs(2)).await;

        let mm = m.lock().await;
        assert!(*mm == 1);
        drop(mm);
    }

    #[tokio::test]
    async fn watch_short() {
        let (tx, m) = setup().await;

        sleep(Duration::from_millis(50)).await;
        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        let _ = tx.send(Some(Local::now() + Duration::from_secs(3)));
        let t = Some(Local::now());

        sleep(Duration::from_secs(2)).await;

        let mm = m.lock().await;
        assert!(*mm == 0);
        drop(mm);

        let _ = tx.send(t);

        sleep(Duration::from_millis(50)).await;
        let mm = m.lock().await;
        assert!(*mm == 1);
        drop(mm);
    }
}

use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
struct Timer {
    tx: WatchTx,
}

type Time = Option<DateTime<Local>>;
type WatchTx = watch::Sender<Time>;
type WatchRx = watch::Receiver<Time>;
type WatchTxResult = Result<(), watch::error::SendError<Time>>;

impl Timer {
    async fn new() -> Self
where {
        let (tx, rx) = watch::channel(None);
        let t = Timer { tx };
        let _ = tokio::spawn(async move { Timer::watch(rx).await });
        t
    }

    fn send(&self, time: Option<DateTime<Local>>) -> WatchTxResult {
        self.tx.send(time)
    }

    async fn watch(mut rx: WatchRx) {
        loop {
            let t = rx.borrow_and_update().clone();

            match t {
                None => {}
                Some(time) => {
                    let dt = time - Local::now();
                    match dt.to_std() {
                        Ok(dur) => {}
                        Err(_) => {}
                    }
                }
            }
        }
    }
}
