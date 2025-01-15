use std::future::{Future, IntoFuture};

use chrono::{DateTime, Local, TimeDelta};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::{sync::watch, time::sleep};

use std::pin::Pin;
use std::task::{Context, Poll};

// We need to be able to poke at the timer while it is held by some thread...
pub type Time = Option<DateTime<Local>>;
pub type TimeTx = watch::Sender<Time>;
type TimeRx = watch::Receiver<Time>;

pub struct TimerLapsedError;

pub struct TimerEditor {
    time_tx: TimeTx,
}

impl TimerEditor {
    pub fn set(&self, time: DateTime<Local>) -> Result<(), TimerLapsedError> {
        self.time_tx.send(Some(time)).map_err(|_| TimerLapsedError)
    }
    pub fn add(&self, delta: TimeDelta) {
        self.time_tx.send_modify(|time| match time.as_mut() {
            Some(time) => *time += delta,
            None => {}
        });
    }
    pub fn cancel(&self) {
        let _ = self.time_tx.send(None);
    }
}

/// A Timer that can be awaited, dropped, or adjusted
pub struct Timer {
    join: JoinHandle<bool>,
    time_tx: TimeTx,
}

impl Timer {
    pub fn new(time: DateTime<Local>) -> Self {
        let (time_tx, time_rx) = watch::channel(Some(time));
        let join = tokio::spawn(Self::timer_internal(time_rx));
        Self { join, time_tx }
    }

    async fn timer_internal(mut rx: TimeRx) -> bool {
        let mut time_payload;
        loop {
            time_payload = rx.borrow_and_update().clone();
            let Some(time) = time_payload else {
                break false; // None sent -> Cancel
            };
            let t = (time - Local::now()).to_std();
            match t {
                Ok(dur) => tokio::select!(
                    _ = rx.changed() => {}
                    _ = sleep(dur) => {break true}
                ),
                Err(_) => break true,
            }
        }
    }

    pub fn editor(&self) -> TimerEditor {
        TimerEditor {
            time_tx: self.time_tx.clone(),
        }
    }
}

/// Allow awaiting a &mut timer
/// ```
/// use crate::engine::timer::Timer;
/// use std::time::Duration;
/// let mut timer = Timer::new(Duration::from_secs(5));
/// (&mut timer).await
/// ```
impl Future for Timer {
    type Output = bool;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().join)
            .poll(cx)
            .map(|r| r.unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::Timer;

    use chrono::{DateTime, Local, TimeDelta};
    use std::time::Duration;

    fn soon(ms: i64) -> DateTime<Local> {
        return Local::now() + TimeDelta::milliseconds(ms);
    }

    async fn sleep(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    #[tokio::test]
    async fn basic() {
        let timer = Timer::new(soon(200));
        assert!(tokio::select! {
            t = timer => t,
            _ = sleep(300) => false,
        });
    }

    #[tokio::test]
    async fn cancel() {
        let timer = Timer::new(soon(200));
        let edit = timer.editor();
        edit.cancel();
        assert!(tokio::select! {
            t = timer => !t,
            _ = sleep(50) => false,
        });
    }

    #[tokio::test]
    async fn set() {
        let mut timer = Timer::new(soon(200));
        let edit = timer.editor();
        edit.set(soon(400));
        assert!(tokio::select! {
            _ = &mut timer => false,
            _ = sleep(200) => true
        });
        edit.set(soon(-100));
        assert!(tokio::select! {
            t = timer => t,
            _ = sleep(50) => false
        });
    }

    #[tokio::test[]]
    async fn add() {
        let mut timer = Timer::new(soon(100));
        let edit = timer.editor();

        for _ in 0..5 {
            edit.add(TimeDelta::milliseconds(200));
            assert!(tokio::select! {
                _ = &mut timer => false,
                _ = sleep(200) => true,
            });
        }
        edit.add(TimeDelta::milliseconds(-200));
        assert!(tokio::select! {
            t = timer => true,
            _ = sleep(200) => false,
        });
    }
}
