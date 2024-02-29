// A timer implementation.

use crate::base::{Choice, ID};
use crate::core::{send_action, Action};
use crate::interface::CommandTx;

use chrono;
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


Hmmm there is an issue with Serialization.
How do we get a timer's data in the synchronous serialize function?

We need some kind of non-blocking system for it...

First idea is we just have a non mutexed struct that holds the data and we can serialize that.
Another idea is that the timer info is stored higher up?
We could have election info stored in Phase::Day. It is updated whenever we update the timer.
In addition, the timer itself then doesn't really need to store that data visibly???


Idea: just fold the timer into the core thread. The core thread is already holding
state and already has the channels protecting it.
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timer<PID: ID> {
    pub end_time: chrono::DateTime<chrono::Local>,
    pub data: Action<PID>,
}

impl<PID: ID> Timer<PID> {
    // Or just return an action?
    pub async fn check(&self) -> Option<Action<PID>> {
        let now = chrono::Local::now();
        if now > self.end_time {
            Some(self.data.clone())
        } else {
            None
        }
    }
}
