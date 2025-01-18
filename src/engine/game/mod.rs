mod builder;

use crate::engine::{interface::Event, state::Choice};

use super::{
    interface::{Action, ActionRx, ActionTx, EventRx, EventTx},
    state::State,
    timer::{Timer, TimerEditor},
};
use anyhow::Result;
use chrono::{DateTime, Local, TimeDelta};
use std::{sync::Arc, time::Duration};
use tokio::sync::{
    broadcast::error::RecvError, oneshot, Mutex as AsyncMutex, Notify, RwLock, TryLockError,
};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct Game {
    state: Arc<RwLock<State>>,
    action_tx: ActionTx,
    action_rx: AsyncMutex<ActionRx>,
    event_tx: EventTx,
    //
}

/*
Would we ever not want to use the Start method to start the game?

*/

impl Game {
    fn new(state: State, (action_tx, action_rx): (ActionTx, ActionRx), event_tx: EventTx) -> Self {
        let state = Arc::new(RwLock::new(state));
        let action_rx = AsyncMutex::new(action_rx);
        Self {
            state,
            action_tx,
            action_rx,
            event_tx,
        }
    }

    /// Get a broadcast rx subscribed to this game
    pub fn event_rx(&self) -> EventRx {
        self.event_tx.subscribe()
    }
    /// Get a mpsc Sender to send actions to this game
    pub fn action_tx(&self) -> ActionTx {
        self.action_tx.clone()
    }

    // pub fn quit(self: Arc<Self>) {
    //     self.quit.notify_one()
    // }

    /// Consumes self and returns Arc, as only shared refs can be used after starting
    async fn start(self) -> Result<Arc<Self>> {
        let (resp, resp_rx) = oneshot::channel();
        let arc_self = Arc::new(self);
        let s = arc_self.clone();
        tokio::spawn(s.run(resp));
        resp_rx.await?;
        Ok(arc_self)
    }

    #[tracing::instrument(skip_all)]
    async fn run(self: Arc<Self>, resp: oneshot::Sender<Result<(), TryLockError>>) {
        match self.clone().action_rx.try_lock() {
            Ok(mut action_rx) => {
                resp.send(Ok(()));
                // Do other init here?
                self.clone().action_handler(&mut action_rx).await;
            }
            Err(e) => {
                resp.send(Err(e));
            }
        }
    }

    #[tracing::instrument]
    async fn action_handler(self: Arc<Self>, action_rx: &mut ActionRx) {
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
                let _ = result;
            }
        }
    }

    fn election_timer(self: Arc<Self>, timer: Timer, choice: Choice, hammer: u64) {
        tokio::spawn(async move {
            if timer.await {
                let mut wstate = self.state.write().await;
                wstate.try_election(choice, hammer, &self.event_tx);
                drop(wstate);
            }
        });
    }

    #[tracing::instrument]
    async fn election_watcher(self: Arc<Self>, mut event_rx: EventRx) {
        fn soon() -> DateTime<Local> {
            return Local::now() + TimeDelta::seconds(10);
        }
        let rstate = self.state.read().await;
        let el = rstate.check_election();
        drop(rstate);
        let mut election = el.map(|el| {
            let timer = Timer::new(soon());
            (el, 0, timer.editor()) // TODO: how to get hammer?
        });
        loop {
            let hammer = match event_rx.recv().await {
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(n)) => {
                    error!(msg = "Missed recv events", ?n);
                    0 // TODO: how to deal with this???
                }
                Ok(Event::Vote {
                    voter,
                    ballot,
                    former,
                }) => {
                    debug!(?voter, ?ballot, ?former);
                    voter
                }
                Ok(Event::Election { .. }) => {
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
            let edit = match (&new_election, &election) {
                (None, Some((_, _, edit))) => Some(edit),
                (Some(new), Some((old, _, edit))) if new != old => Some(edit),
                _ => None,
            };
            if let Some(edit) = edit {
                let _ = edit.cancel(); // Cancel timer
                election = None;
            }
            if let Some(choice) = new_election {
                let timer = Timer::new(soon());
                election = Some((choice.clone(), hammer, timer.editor()));
                self.clone().election_timer(timer, choice, hammer);
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
        let rstate = self.state.read().await;
        let mut dawn_imminent: bool = rstate.check_dawn();
        drop(rstate);
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
                let timer = Timer::new(soon);
                self.clone().dawn_timer(timer);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use anyhow::Result;

    use tokio::{
        sync::{
            oneshot, Mutex, MutexGuard, OwnedMutexGuard, RwLock, RwLockWriteGuard, TryLockError,
        },
        task::JoinHandle,
    };

    use crate::engine::{
        interface::{Action, ActionRx, ActionTx, Error, EventRx, EventTx},
        state::{phase::Phase, players::Players, role::Role, rules::Rules, Choice},
    };

    struct Game {
        state: Arc<RwLock<State>>,

        action_tx: ActionTx,                     // Allow cloning
        action_rx: Arc<Mutex<Option<ActionRx>>>, // Option so we can take it
        event_tx: EventTx,                       // Allow subscribing
    }

    impl Game {
        // How can state be created without roles? Should rolegen be rolled in?
        pub fn new(users: Vec<u64>, rules: Rules) -> Self {
            todo!()
        }

        pub fn new_with_roles(users: Vec<u64>, roles: Vec<u64>, rules: Rules) -> Self {
            todo!()
        }

        pub fn load(state: State) -> Self {
            todo!()
        }

        pub fn action_tx(&self) -> ActionTx {
            self.action_tx.clone()
        }
        pub fn event_rx(&self) -> EventRx {
            self.event_tx.subscribe()
        }

        pub fn start(&self) -> Result<JoinHandle<()>, ()> {
            if let Some(mut action_rx) = self.try_take_rx() {
                let rx_holder = self.action_rx.clone();
                let state = self.state.clone();
                Ok(tokio::spawn(async move {
                    State::action_handler(state, &mut action_rx).await;
                    rx_holder.lock().await.replace(action_rx);
                }))
            } else {
                Err(())
            }
        }

        pub async fn run(&self) -> Result<(), ()> {
            if let Some(mut action_rx) = self.try_take_rx() {
                State::action_handler(self.state.clone(), &mut action_rx).await;
                self.action_rx.lock().await.replace(action_rx);
                Ok(())
            } else {
                Err(())
            }
        }

        fn try_take_rx(&self) -> Option<ActionRx> {
            if let Ok(mut opt) = self.action_rx.try_lock() {
                opt.take()
            } else {
                None
            }
        }
    }

    struct State {
        id: u64,
        day: u32,
        phase: Phase,
        players: Players,
        rules: Rules,
        tx: EventTx,
        undo_timer: Option<JoinHandle<()>>,
    }

    impl State {
        pub fn new(id: u64, registry: Vec<(u64, Role)>, rules: Rules) -> Self {
            todo!()
        }

        // Should validate action be done first?
        pub fn validate_action(&self, action: Action) -> Result<(), Error> {
            todo!()
        }

        pub async fn action_handler(this: Arc<RwLock<Self>>, action_rx: &mut ActionRx) {
            while let Some((action, resp)) = action_rx.recv().await {
                Self::handle_action(&this, action, resp);
            }
        }
        // The game pointer is used for... Event tx, to send events...
        // And for timers to have a handle on the state mutex...
        // It would be nice to do this differently...
        pub async fn handle_action(
            this: &Arc<RwLock<Self>>,
            action: Action,
            resp: oneshot::Sender<Result<(), Error>>,
        ) -> anyhow::Result<()> {
            // TODO: Check if locking read, validating, then on Ok locking write and re-validating would work?
            let mut wstate = this.write().await;
            // TODO: should this return anything else?
            let result = wstate.validate_action(action);
            resp.send(result.clone());

            enum ActionResult {
                Election { choice: Choice, hammer: u64 },
                Dawn,
                None,
            }

            // HANDLE ACTIONS

            let result = match action {
                Action::Vote { voter, ballot } => {
                    // wstate.vote -> Result<Option<(Choice,usize)>>
                    // Ok(Some(Election))
                    let reached_new_election = Some(Some(1));
                    let hammer = voter;

                    if let Some(choice) = reached_new_election {
                        ActionResult::Election { choice, hammer }
                    } else {
                        ActionResult::None
                    }
                }
                Action::Target { actor, choice } => {
                    // ...
                    let night_done = true;
                    if night_done {
                        ActionResult::Dawn
                    } else {
                        ActionResult::None
                    }
                }
                Action::Scheme { killer, mark } => {
                    // ...
                    let night_done = true;
                    if night_done {
                        ActionResult::Dawn
                    } else {
                        ActionResult::None
                    }
                }
                _ => ActionResult::None,
            };

            // IF DAWN OR ELECTION IS DETECTED
            match result {
                ActionResult::Election { choice, hammer } => {
                    let h = tokio::spawn(Self::election_timer(this.clone(), choice, hammer));
                    if let Some(old_h) = wstate.undo_timer.replace(h) {
                        old_h.abort();
                    }
                }
                ActionResult::Dawn => {
                    let h = tokio::spawn(Self::dawn_timer(this.clone()));
                    if let Some(old_h) = wstate.undo_timer.replace(h) {
                        old_h.abort();
                    }
                }
                ActionResult::None => {}
            }

            drop(wstate);

            todo!()
        }

        fn check_election(&self, choice: Choice) -> Result<Option<(Vec<u64>)>> {
            todo!()
        }

        fn election(&mut self, choice: Choice, hammer: u64, voters: Vec<u64>) -> Result<()> {
            todo!()
        }

        fn dawn(&mut self) -> Result<()> {
            todo!()
        }

        async fn election_timer(this: Arc<RwLock<Self>>, choice: Choice, hammer: u64) {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let mut wstate = this.write().await;
            // TODO: Error tracing
            if let Some(users) = wstate.check_election(choice).unwrap() {
                wstate.election(choice, hammer, users).unwrap();
            }
            wstate.undo_timer = None;
        }
        async fn dawn_timer(this: Arc<RwLock<Self>>) {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let mut wstate = this.write().await;
            // TODO: Error tracing
            wstate.dawn().unwrap();
            wstate.undo_timer = None;
        }

        // Why does state need a mutex? Because status, action handler, and timers might contest
        // Should state know about the mutex? Yeah, maybe... It could take itself at every entry
        // Including timers.
        // event broadcast could be done locally... we could even just have it as a function?
        // Hmmm, maybe. Self.tx(event) is a Fn(Event) -> ()
    }

    #[test]
    #[ignore]
    fn typical_use() {
        // Create game with... rules? Players? Roles?
        // Assigned roles could come with rules
        // From a loaded state

        // Id comes from a file, which is then incremented... Don't worry about concurrency for now...
        let users = vec![];
        let rules = Rules {};
        // let roles = vec![];
        let mut game: Game = Game::new(users, rules); // Roles are generated... at start
                                                      // let game: Game = Game::new_with_roles(users, roles, rules);

        // let game: Game = Game::load(state); // require details of chats, etc to resume?

        // let game
        // Should there be some kind of "resume" functionality? Can we assume from_state can handle that?

        // Start the game.
        game.start().expect("Nobody has started this yet");
    }
}
