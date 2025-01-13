use std::future::Future;

use chrono::{DateTime, Local};
use tokio::sync::watch::error::{RecvError, SendError};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::{
    sync::watch::{self},
    task::AbortHandle,
    time::sleep,
};
use tracing::error;

type Time = Option<DateTime<Local>>;
type TimeTx = watch::Sender<Time>;
type TimeRx = watch::Receiver<Time>;

struct Timer<T> {
    time_tx: TimeTx,
    return_rx: mpsc::Receiver<T>,
    handle: AbortHandle,
}

// TODO: Figure out how to pass a method to spawn?

impl<T> Timer<T> {
    pub fn spawn<F, Fut>(t: Option<DateTime<Local>>, f: F) -> Self
    where
        F: (Fn() -> Fut) + Send + 'static,
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
                let _ = return_tx.send(f().await).await;
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
    fn set(&self, dt: Time) -> Result<(), SendError<Time>> {
        self.time_tx.send(dt)
    }
    async fn get(&mut self) -> Option<T> {
        self.return_rx.recv().await
    }

    fn abort(&self) {
        self.handle.abort();
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
        let t = Timer::spawn(None, move || {
            let m2 = m2.clone();
            async move {
                let mut mm = m2.lock().await;
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

        let f = move || {
            let m = m2.clone();
            async move {
                let mut mm = m.lock().await;
                *mm += 1;
                let v = *mm;
                v
            }
        };

        let soon = Local::now() + Duration::from_millis(200);

        let mut t = Timer::spawn(Some(soon), f);

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
