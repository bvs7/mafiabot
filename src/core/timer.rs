// A timer implementation.

use core::future::Future;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};

/*
Timers:
- Are given a remaining duration and a callback
- Can be started, cancelled, and can have their duration changed.

just include data in the callback?


*/
struct TimerData {
    end_time: Instant,
    cancelled: bool,
    finished: bool,
}

pub struct Timer {
    data: Arc<Mutex<TimerData>>,
}

impl Timer {
    pub fn new<F>(end_time: Instant, precision: Duration, callback: F) -> Timer
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let cancelled = false;
        let finished = false;
        let data = Arc::new(Mutex::new(TimerData {
            end_time,
            cancelled,
            finished,
        }));
        let timer = Timer {
            data: Arc::clone(&data),
        };
        let _ = tokio::spawn(async move {
            loop {
                tokio::time::sleep(precision).await;
                {
                    let mut td = data.lock().await;
                    if td.cancelled {
                        td.finished = true;
                        break;
                    }
                    if Instant::now() >= td.end_time {
                        callback.await;
                        td.finished = true;
                        break;
                    }
                }
            }
        });
        timer
    }

    pub async fn cancel(&self) {
        let mut td = self.data.lock().await;
        td.cancelled = true;
    }

    pub async fn is_cancelled(&self) -> bool {
        let td = self.data.lock().await;
        return td.cancelled;
    }

    pub async fn set_end_time(&self, end_time: Instant) {
        let mut td = self.data.lock().await;
        td.end_time = end_time;
    }

    pub async fn get_end_time(&self) -> Instant {
        let td = self.data.lock().await;
        return td.end_time.clone();
    }

    pub async fn is_finished(&self) -> bool {
        let td = self.data.lock().await;
        return td.finished;
    }
}

impl Debug for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timer_basic() {
        let data = Arc::new(Mutex::new(0));
        assert_eq!(*data.lock().await, 0);

        let now = Instant::now();
        let elapse = Duration::from_millis(200);
        let inc = Duration::from_millis(10);
        let end_time = now + elapse;

        let data_ref = Arc::clone(&data);
        let timer = Timer::new(end_time, inc, async move {
            *data_ref.lock().await = 1;
        });

        tokio::time::sleep(inc).await;

        assert_eq!(*data.lock().await, 0);
        assert_eq!(timer.is_finished().await, false);

        tokio::time::sleep(elapse + inc).await;
        assert_eq!(*data.lock().await, 1);
        assert_eq!(timer.is_finished().await, true);
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let data = Arc::new(Mutex::new(0));
        assert_eq!(*data.lock().await, 0);

        let now = Instant::now();
        let elapse = Duration::from_millis(200);
        let inc = Duration::from_millis(10);
        let end_time = now + elapse;

        let data_ref = Arc::clone(&data);
        let timer = Timer::new(end_time, inc, async move {
            *data_ref.lock().await = 1;
        });

        assert_eq!(*data.lock().await, 0);

        tokio::time::sleep(elapse / 2).await;

        assert_eq!(*data.lock().await, 0);
        assert_eq!(timer.is_finished().await, false);
        assert_eq!(timer.is_cancelled().await, false);
        timer.cancel().await;
        assert_eq!(timer.is_cancelled().await, true);

        tokio::time::sleep(inc * 2).await;
        assert_eq!(timer.is_finished().await, true);

        tokio::time::sleep(elapse).await;
        assert_eq!(*data.lock().await, 0);
    }

    #[tokio::test]
    async fn test_timer_set_end_time() {
        let data = Arc::new(Mutex::new(0));
        assert_eq!(*data.lock().await, 0);

        let start = Instant::now();
        let elapse = Duration::from_millis(200);
        let inc = Duration::from_millis(10);

        let end_time1 = start + elapse;
        let end_time2 = start + elapse * 2;
        let end_time3 = start + elapse * 3;

        let data1 = Arc::clone(&data);
        let timer1 = Timer::new(end_time1, inc, async move {
            *data1.lock().await = 1;
        });

        let data2 = Arc::clone(&data);
        let timer2 = Timer::new(end_time2, inc, async move {
            *data2.lock().await = 2;
        });

        let data3 = Arc::clone(&data);
        let timer3 = Timer::new(end_time3, inc, async move {
            *data3.clone().lock().await = 3;
        });

        // Basic delay
        tokio::time::sleep(inc).await;

        assert_eq!(
            *data.lock().await,
            0,
            "Data did not stay 0 after starting timers"
        );

        // Change timer1 to end at the same time as timer3
        timer1.set_end_time(end_time3).await;

        // Wait for timer2 to proc (1 and three should not proc yet!)
        tokio::time::sleep(elapse * 2).await;

        assert_eq!(*data.lock().await, 2);

        timer3.set_end_time(start).await; // should proc immediately (within precision...)
        tokio::time::sleep(inc * 2).await;

        assert_eq!(*data.lock().await, 3);

        tokio::time::sleep(elapse * 2).await;

        assert_eq!(*data.lock().await, 1);
    }
}
