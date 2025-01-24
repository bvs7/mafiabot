use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::{DateTime, Local, OutOfRangeError};
use tokio::{
    sync::{MutexGuard, TryLockError},
    time::timeout,
};
use toml::value::Date;

enum Error {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Pid(u64);

type Choice = Option<Pid>;
type Ballot = Option<Choice>;

// These are using Pid, does this mean they are validated?

#[derive(Debug, Clone)]
struct Vote {
    voter: Pid,
    ballot: Ballot,
}

#[derive(Debug, Clone)]
struct Target {
    actor: Pid,
    choice: Choice,
}

#[derive(Debug, Clone)]
enum Action {
    Vote(Vote),
    Target(Target),
}

struct Status {}

#[derive(Debug, Clone)]
enum Event {}

struct Timeout {}

impl From<OutOfRangeError> for Timeout {
    fn from(_: OutOfRangeError) -> Self {
        Timeout {}
    }
}
impl From<tokio::time::error::Elapsed> for Timeout {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Timeout {}
    }
}

struct Game {
    state: tokio::sync::RwLock<State>,
    action_rx: tokio::sync::Mutex<tokio::sync::mpsc::Receiver<Action>>,
    action_tx: tokio::sync::mpsc::Sender<Action>,
    event_tx: tokio::sync::broadcast::Sender<Event>,
    alarm_time: Option<chrono::DateTime<chrono::Local>>,
}

impl Game {
    async fn new(mut state: State) -> Self {
        let (action_tx, action_rx) = tokio::sync::mpsc::channel(100);
        let (event_tx, _) = tokio::sync::broadcast::channel(100);
        let action_rx = tokio::sync::Mutex::new(action_rx);
        state.event_tx = Some(event_tx.clone());
        let alarm_time = None;
        Self {
            state: tokio::sync::RwLock::new(state),
            action_rx,
            action_tx,
            event_tx,
            alarm_time,
        }
    }

    fn action_tx(&self) -> tokio::sync::mpsc::Sender<Action> {
        self.action_tx.clone()
    }

    fn event_rx(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.event_tx.subscribe()
    }

    fn start(self) -> Arc<Self> {
        let game = Arc::new(self);
        let g = game.clone();
        tokio::spawn(async move { g.run().await });
        game
    }
    async fn run(&self) -> Result<(), TryLockError> {
        let mut action_rx = self.action_rx.try_lock()?;
        loop {
            match self.next_action(&mut action_rx).await {
                Ok(Some((action, resp))) => {
                    let result = self.handle_action(action).await;
                    let _ = resp.send(result);
                }
                Ok(None) => {
                    break;
                }
                Err(Timeout {}) => {}
            }
            self.update().await;
        }
        Ok(())
    }

    async fn next_action(
        &self,
        action_rx: &mut tokio::sync::mpsc::Receiver<Action>,
    ) -> Result<Option<(Action, tokio::sync::oneshot::Sender<Result<(), Error>>)>, Timeout> {
        let dur = self.next_timeout()?;
        match timeout(dur, action_rx.recv()).await? {
            Some(action) => {
                let (resp, rx) = tokio::sync::oneshot::channel();
                Ok(Some((action, resp)))
            }
            None => Ok(None),
        }
    }
    fn next_timeout(&self) -> Result<Duration, OutOfRangeError> {
        if let Some(alarm_time) = self.alarm_time {
            let dur = (alarm_time - Local::now()).to_std()?;
            return Ok(dur);
        } else {
            return Ok(Duration::MAX);
        }
    }

    /// Check validity with just read, then try writing.
    async fn handle_action(&self, action: Action) -> Result<(), Error> {
        let rstate = self.state.read().await;
        rstate.validate_action(&action)?;
        drop(rstate);
        let mut wstate = self.state.write().await;
        wstate.validate_action(&action)?;
        wstate.action(action);
        Ok(())
    }
    async fn update(&self) -> Option<DateTime<Local>> {
        // Check for updates
        todo!()
    }

    async fn status(&self) -> Status {
        let rstate = self.state.read().await;
        let status = rstate.status();
        status
    }
}

#[derive(Debug, Clone)]
enum Team {}

#[derive(Debug, Clone)]
enum Phase {
    Init,
    Day {
        votes: HashMap<Pid, Choice>,
        blocks: HashMap<Pid, Vec<Pid>>,
        elect: Option<(Choice, Pid, DateTime<Local>)>,
    },
    Night {
        targets: HashMap<Pid, Choice>,
        scheme: Option<(Pid, Choice)>,
    },
    Eclipse,
    End {
        winner: Team,
    },
}

struct State {
    day: u32,
    phase: Phase,
    event_tx: Option<tokio::sync::broadcast::Sender<Event>>,
}

impl State {
    fn status(&self) -> Status {
        todo!()
    }

    // How could this validation work?
    fn validate_action(&self, action: &Action) -> Result<(), Error> {
        todo!()
    }

    /// Action must not be able to fail. Allow panic on error in here?
    fn action(&mut self, action: Action) {
        todo!()
    }

    fn vote(&mut self, voter: Pid, ballot: Ballot) {
        //
    }

    fn send(&self, event: Event) {
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(event);
        }
    }
}

enum Update {
    Election {
        choice: Choice,
        hammer: Pid,
        voters: Vec<Pid>,
    },
    ElectionImminent {
        choice: Choice,
        hammer: Pid,
        time: DateTime<Local>,
    },
    ElectionAverted,
    Dawn,
    DawnImminent {
        time: DateTime<Local>,
    },
    Vengeance {
        avenger: Pid,
        victim: Pid,
        hammer: Pid,
    },
}

impl State {
    fn poll_update(&self) -> () {
        // Check for an update to phase?
        match &mut self.phase {
            Phase::Day { votes, elect, .. } => {
                // Check for an election...
            }
            Phase::Night {
                pend_dawn: Some(time),
                ..
            } if *time < Local::now() => {
                result = Some(UpdateResult::Dawn);
            }
            Phase::Night {
                targets,
                scheme,
                pend_dawn,
            } => {
                let mut ready = true;
                if scheme.is_none() {
                    ready = false;
                }
                for (pid, role) in self.players.alive() {
                    if role.is_targeting() && targets.get(&pid).is_none() {
                        ready = false;
                    }
                }
                if ready {
                    let time = Local::now() + DAWN_DELAY;
                    *pend_dawn = Some(time);
                    result = Some(UpdateResult::DawnImminent(time));
                }
            }
            Phase::Eclipse {
                avenger,
                hammer,
                vote: Some(victim),
                ..
            } => {
                result = Some(UpdateResult::Vengeance(*avenger, *victim, *hammer));
            }
            _ => {}
        }
    }
}
