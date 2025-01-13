use std::future::Future;

use chrono::{DateTime, Local};
use tokio::sync::watch::error::{RecvError, SendError};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::{
    sync::watch::{self},
    task::AbortHandle,
    time::sleep,
};
use tracing::error;

type Time = Option<DateTime<Local>>;
type TimeTx = watch::Sender<Time>;
type TimeRx = watch::Receiver<Time>;

pub struct Timer<T> {
    time_tx: TimeTx,
    return_rx: mpsc::Receiver<T>,
    handle: AbortHandle,
}

// TODO: Figure out how to pass a method to spawn?
// New Generic, pass in a self clone? Can't be reference...
// But we could pass the input into spawn then just have the fn call on it each time...

impl<T> Timer<T> {
    pub fn spawn<S, F, Fut>(s: S, t: Option<DateTime<Local>>, f: F) -> Self
    where
        S: Send + 'static,
        F: (Fn(&S) -> Fut) + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (time_tx, time_rx) = watch::channel(t);
        let time_tx2 = time_tx.clone();
        let (return_tx, return_rx) = mpsc::channel(100);
        let handle = tokio::spawn(async move {
            let mut time_rx = time_rx;
            while Self::timer(&mut time_rx).await.is_ok() {
                let _ = time_tx2.send(None);
                let _ = return_tx.send(f(&s).await).await;
            }
        })
        .abort_handle();

        Self {
            time_tx,
            return_rx,
            handle,
        }
    }
    async fn timer(rx: &mut TimeRx) -> Result<(), RecvError> {
        let mut t = rx.borrow_and_update().clone();
        loop {
            if let Some(time) = t {
                tokio::select! {
                    r = rx.changed() => {r?},
                    _ = Self::wait_until(time) => {return Ok(())},
                }
            } else {
                rx.changed().await?
            }

            t = rx.borrow_and_update().clone();
        }
    }
    async fn wait_until(time: DateTime<Local>) {
        if let Ok(dur) = (time - Local::now()).to_std() {
            sleep(dur).await;
        }
    }
    pub fn set(&self, dt: Time) -> Result<(), SendError<Time>> {
        self.time_tx.send(dt)
    }
    pub async fn get(&mut self) -> Option<T> {
        self.return_rx.recv().await
    }

    pub fn abort(&self) {
        self.handle.abort();
    }
}

// new timer

pub struct Timer2 {
    join: JoinHandle<()>,
    time_tx: watch::Sender<DateTime<Local>>,
}

pub struct TimerDropped;

pub fn timer(time: DateTime<Local>) -> JoinHandle<()> {
    timer_with_editor(time).0
}

pub fn timer_with_editor(
    time: DateTime<Local>,
) -> (JoinHandle<()>, watch::Sender<DateTime<Local>>) {
    let (tx, mut rx) = watch::channel(time);
    let h = tokio::spawn(async move { timer_internal(rx).await });
    (h, tx)
}

async fn timer_internal(mut rx: watch::Receiver<DateTime<Local>>) -> () {
    let mut time = rx.borrow_and_update().clone();
    let mut r = Some(rx);
    loop {
        let t = (time - Local::now()).to_std();

        let result = match (&mut r, t) {
            (_, Err(_)) => return,
            (None, Ok(dur)) => {
                sleep(dur).await;
                return;
            }
            (Some(rx), Ok(dur)) => {
                tokio::select!(
                    result = rx.changed() => {
                        time = rx.borrow_and_update();
                        result
                    }, // if tx is dropped, just continue...
                    _ = sleep(dur) => {return ()},
                )
            }
        };
        if result.is_err() {
            r = None;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::engine::timer::*;
    use chrono::Local;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::time::{sleep, timeout, Duration};
    use tracing::debug;
    use tracing_test::traced_test;

    fn setup() -> (Timer<i32>, Arc<Mutex<i32>>) {
        let m = Arc::new(Mutex::new(0));
        let m2 = m.clone();
        let t = Timer::spawn(m2, None, move |m| {
            let m = m.clone();
            async move {
                let mut mm = m.lock().await;
                *mm = 1;
                drop(mm);
                3
            }
        });
        return (t, m);
    }

    async fn cmp_mutex(m: &Arc<Mutex<i32>>, v: i32) -> bool {
        let mm = m.lock().await;
        let r = *mm == v;
        drop(mm);
        r
    }

    #[traced_test]
    #[tokio::test]
    async fn get_return() -> anyhow::Result<()> {
        let (mut t, _m) = setup();
        t.set(Some(Local::now() + Duration::from_millis(200)))?;
        let result = timeout(Duration::from_millis(300), t.get()).await;
        debug!(?result);
        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn basic() -> anyhow::Result<()> {
        let (t, m) = setup();

        assert!(cmp_mutex(&m, 0).await);
        sleep(Duration::from_millis(50)).await;

        t.set(Some(Local::now() + Duration::from_millis(200)))?;

        sleep(Duration::from_millis(50)).await;
        assert!(cmp_mutex(&m, 0).await);
        sleep(Duration::from_millis(400)).await;
        assert!(cmp_mutex(&m, 1).await);
        Ok(())
    }

    #[tokio::test]
    async fn cancel() -> anyhow::Result<()> {
        let (t, m) = setup();
        assert!(cmp_mutex(&m, 0).await);

        let _ = t.set(Some(Local::now() + Duration::from_millis(200)));
        sleep(Duration::from_millis(50)).await;
        assert!(cmp_mutex(&m, 0).await);

        let _ = t.set(None);
        sleep(Duration::from_millis(400)).await;
        assert!(cmp_mutex(&m, 0).await);
        Ok(())
    }

    #[tokio::test]
    async fn shorten() -> anyhow::Result<()> {
        let (t, m) = setup();
        assert!(cmp_mutex(&m, 0).await);

        let _ = t.set(Some(Local::now() + Duration::from_millis(400)));
        sleep(Duration::from_millis(10)).await;
        assert!(cmp_mutex(&m, 0).await);

        let _ = t.set(Some(Local::now() - Duration::from_millis(20)));
        sleep(Duration::from_millis(10)).await;
        assert!(cmp_mutex(&m, 1).await);
        Ok(())
    }

    #[tokio::test]
    async fn extend() -> anyhow::Result<()> {
        let (t, m) = setup();
        assert!(cmp_mutex(&m, 0).await);

        let _ = t.set(Some(Local::now() + Duration::from_millis(100)));
        sleep(Duration::from_millis(10)).await;
        assert!(cmp_mutex(&m, 0).await);

        let _ = t.set(Some(Local::now() + Duration::from_millis(300)));
        sleep(Duration::from_millis(200)).await;
        assert!(cmp_mutex(&m, 0).await);
        sleep(Duration::from_millis(200)).await;
        assert!(cmp_mutex(&m, 1).await);
        Ok(())
    }

    #[tokio::test]
    async fn multiple() -> anyhow::Result<()> {
        let m = Arc::new(Mutex::new(0));
        let m2 = m.clone();

        let f = |m2: &Arc<Mutex<i32>>| {
            let m = m2.clone();
            async move {
                let mut mm = m.lock().await;
                *mm += 1;
                let v = *mm;
                v
            }
        };

        let soon = Local::now() + Duration::from_millis(200);

        let mut t = Timer::spawn(m2, Some(soon), f);

        assert!(cmp_mutex(&m, 0).await);
        sleep(Duration::from_millis(300)).await;

        let r = t.get().await;
        assert!(r == Some(1));

        t.set(Some(soon))?;

        sleep(Duration::from_millis(10)).await;
        let r = t.get().await;
        assert!(r == Some(2));

        Ok(())
    }
}
