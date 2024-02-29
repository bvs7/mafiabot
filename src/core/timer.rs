// A timer implementation.

use crate::base::{Choice, ID};
use crate::core::{send_action, Action};
use crate::interface::CommandTx;

use core::future::Future;
use serde::de;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};

/*
Timers:
- Are given a remaining duration and a callback
- Can be started, cancelled, and can have their duration changed.

just include data in the callback?
*/

#[derive(Debug, Clone, Serialize)]
pub enum TimerCallback<PID: ID> {
    Elect {
        candidate: Choice<PID>,
        hammer: PID,
    },
    Dawn(),
    #[serde(skip)]
    Test {
        data_ref: Arc<Mutex<i32>>,
        data: i32,
    },
}

#[derive(Clone, Debug)]
pub struct TimerData<PID: ID> {
    end_time: Instant,
    cancelled: bool,
    finished: bool,
    callback: TimerCallback<PID>,
}

#[derive(Clone)]
pub struct Timer<PID: ID> {
    t: Arc<Mutex<TimerData<PID>>>,
}

impl<PID: ID> Timer<PID> {
    pub fn new(end_time: Instant, callback: TimerCallback<PID>) -> Timer<PID> {
        let cancelled = false;
        let finished = false;
        let t = Arc::new(Mutex::new(TimerData {
            end_time,
            cancelled,
            finished,
            callback,
        }));
        let timer = Timer { t };

        timer
    }

    pub async fn start(&self, cmd_tx: CommandTx<PID>) {
        let t = Arc::clone(&self.t);
        let precision = Duration::from_millis(100);
        let _ = tokio::spawn(async move {
            loop {
                tokio::time::sleep(precision).await;
                {
                    let mut td = t.lock().await;
                    if td.cancelled {
                        td.finished = true;
                        break;
                    }
                    if Instant::now() >= td.end_time {
                        match &td.callback {
                            TimerCallback::Elect { candidate, hammer } => {
                                send_action(
                                    &cmd_tx,
                                    Action::Elect {
                                        candidate: *candidate,
                                        hammer: *hammer,
                                    },
                                )
                                .await
                            }
                            TimerCallback::Dawn() => send_action(&cmd_tx, Action::Dawn).await,
                            TimerCallback::Test { data_ref, data } => {
                                *data_ref.lock().await = *data;
                                Ok(())
                            }
                        }
                        .expect("Error in timer callback: ");
                        td.finished = true;
                        break;
                    }
                }
            }
        });
    }

    pub async fn cancel(&self) {
        let mut td = self.t.lock().await;
        td.cancelled = true;
    }

    pub async fn is_cancelled(&self) -> bool {
        let td = self.t.lock().await;
        return td.cancelled;
    }

    pub async fn set_end_time(&self, end_time: Instant) {
        let mut td = self.t.lock().await;
        td.end_time = end_time;
    }

    pub async fn get_end_time(&self) -> Instant {
        let td = self.t.lock().await;
        return td.end_time.clone();
    }

    pub async fn is_finished(&self) -> bool {
        let td = self.t.lock().await;
        return td.finished;
    }
}

impl<PID: ID> Debug for Timer<PID> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: DON'T BLOCK?!
        let td = self.t.blocking_lock();
        write!(
            f,
            "Timer {{ end_time: {:?}, cancelled: {:?}, finished: {:?}, callback: {:?} }}",
            td.end_time, td.cancelled, td.finished, td.callback
        )
    }
}

impl<PID: ID> Serialize for Timer<PID> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let td = self.t.blocking_lock();
        let until_end_time = td.end_time - Instant::now();
        let end_system_time = SystemTime::now() + until_end_time;
        let mut state = serializer.serialize_struct("Timer", 4)?;
        state.serialize_field("end_time", &end_system_time)?;
        match &td.callback {
            TimerCallback::Elect { candidate, hammer } => {
                let candidate: Option<PID> = (*candidate).into();
                state.serialize_field("candidate", &candidate)?;
                state.serialize_field("hammer", &hammer)?;
            }
            TimerCallback::Dawn() => {
                state.serialize_field("callback", "Dawn")?;
            }
            _ => {
                state.serialize_field("callback", "Test")?;
            }
        }
        state.end()
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
        let elapse = Duration::from_millis(500);
        let inc = Duration::from_millis(100);
        let end_time = now + elapse;

        let (dummy_tx, _) = crate::interface::command_channel::<u32>();

        let data_ref = Arc::clone(&data);
        let timer = Timer::new(end_time, TimerCallback::Test { data_ref, data: 1 });
        timer.start(dummy_tx.clone()).await;

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
        let elapse = Duration::from_millis(500);
        let inc = Duration::from_millis(100);
        let end_time = now + elapse;

        let (dummy_tx, _) = crate::interface::command_channel::<u32>();

        let data_ref = Arc::clone(&data);

        let timer = Timer::new(end_time, TimerCallback::Test { data_ref, data: 1 });
        timer.start(dummy_tx.clone()).await;

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
        let elapse = Duration::from_millis(500);
        let inc = Duration::from_millis(100);
        let (dummy_tx, _) = crate::interface::command_channel::<u32>();

        let end_time1 = start + elapse;
        let end_time2 = start + elapse * 2;
        let end_time3 = start + elapse * 3;

        let timer1 = Timer::new(
            end_time1,
            TimerCallback::Test {
                data_ref: data.clone(),
                data: 1,
            },
        );
        timer1.start(dummy_tx.clone()).await;

        let timer2 = Timer::new(
            end_time2,
            TimerCallback::Test {
                data_ref: data.clone(),
                data: 2,
            },
        );
        timer2.start(dummy_tx.clone()).await;
        let timer3 = Timer::new(
            end_time3,
            TimerCallback::Test {
                data_ref: data.clone(),
                data: 3,
            },
        );
        timer3.start(dummy_tx.clone()).await;

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
