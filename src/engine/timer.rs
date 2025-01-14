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

/// A Timer that can be awaited, dropped, or adjusted
pub struct Timer {
    join: JoinHandle<bool>,
    time_tx: TimeTx,
}

impl Timer {
    pub fn new(time: DateTime<Local>) -> (Self, TimeTx) {
        let (time_tx, time_rx) = watch::channel(Some(time));
        let join = tokio::spawn(Self::timer_internal(time_rx));
        let tx = time_tx.clone();
        (Self { join, time_tx }, tx)
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

    fn get_time_tx(&self) -> TimeTx {
        self.time_tx.clone()
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
/*
#[cfg(test)]
mod test {
    use super::Timer;

    use chrono::{Local, TimeDelta};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn basic() {
        let mut timer = Timer::new(Local::now() + Duration::from_millis(200));
        assert!(tokio::select! {
            _ = &mut timer => true,
            _ = sleep(Duration::from_millis(300)) => false,
        });
    }

    #[tokio::test]
    async fn dropping() {
        let timer = Timer::new(Local::now() + Duration::from_millis(100));
        drop(timer);

        sleep(Duration::from_millis(500)).await;
    }

    #[tokio::test]
    async fn set() {
        let mut timer = Timer::new(Local::now() + Duration::from_millis(200));
        sleep(Duration::from_millis(100)).await;

        for _ in 0..10 {
            timer.add(TimeDelta::milliseconds(200));
            assert!(tokio::select! {
                _ = sleep(Duration::from_millis(200)) => true,
                _ = &mut timer => false,
            });
        }

        timer.add(TimeDelta::milliseconds(-200));

        assert!(tokio::select! {
            _ = &mut timer => true,
            _ = sleep(Duration::from_millis(10)) => false,
        });
    }
}
*/
