use std::{future::Future, marker::PhantomData, sync::mpsc};

use chrono::{DateTime, Local};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::{
    sync::watch::{self},
    task::{AbortHandle, JoinHandle},
    time::{sleep, timeout, Duration},
};
use tracing::error;

type TimeTx = watch::Sender<Option<(DateTime<Local>)>>;
type TimeRx = watch::Receiver<Option<(DateTime<Local>)>>;

/// A timer object that can be spawned with a callback fn of some sort
///
/// We want to spawn the timer, then be able to set the time with tx (schedule), then know it will call the given future when its finished
///
/// ```
/// use crate::engine::timer::*;
/// use tokio::sync::Mutex;
/// use std::sync::Arc;
/// use chrono::{DateTime, Local};
/// use tokio::time::{sleep, Duration};
///
/// let m = Arc::new(Mutex::new(0));
/// let m2 = m.clone();
/// let t = Timer::spawn(async move {
///     let mm = m2.lock().await;
///     *mm = 1;
///     drop(mm);
/// });
///
/// let mm = m2.lock().await;
/// assert!(*mm == 0);
/// drop(mm);
///
/// sleep(Duration::from_millis(50)).await;
///
/// t.time_tx.send(Some(Local::now() + Duration::from_millis(200)));
///
/// sleep(Duration::from_millis(50)).await;
///
/// let mm = m2.lock().await;
/// assert!(*mm == 0);
/// drop(mm);
///
/// sleep(Duration::from_millis(400)).await;
///
/// let mm = m2.lock().await;
/// assert!(*mm == 1);
/// drop(mm);
///
/// ```
///
struct Timer<T> {
    time_tx: TimeTx,
    return_rx: mpsc::Receiver<T>,
    handle: AbortHandle,
}

impl<T> Timer<T> {
    fn spawn<F>(future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (time_tx, time_rx) = watch::channel(None);
        let (return_tx, return_rx) = mpsc::channel();
        let handle = tokio::spawn(async move {
            let mut time_rx = time_rx;
            if Timer::<T>::timer(&mut time_rx).await {
                let _ = return_tx.send(future.await);
            }
        })
        .abort_handle();

        Self {
            time_tx,
            return_rx,
            handle,
        }
    }
    async fn timer(rx: &mut TimeRx) -> bool {
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
}
