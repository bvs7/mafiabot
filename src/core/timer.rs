// A timer implementation.

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

/*
Timers:
- Are given a remaining duration and a callback
- Can be started, cancelled, and can have their duration changed.

There will need to be wrappers for timers that include what type of timer it is, the data it is associated with, and the destined end time

One issue is in the function being passed in. How is it related to the Core object as a whole? We can't move in the entire core. What we can do is move in an Action Queue.

But do we want an action queue for everything?

Maybe make the Action Queue transparent. That is, the user calls a function which sends to the queue, waits for the response, then responds? At that point, don't even make it a queue?

hmmm this seems tough.

Ok, so we want to have:
- A hammer vote is cast by the Controller calling the Core's vote function. No errors are returned so that request finishes
- The Controller polls the Event Queue and handles those events associated with the vote
- A countdown is started by the Core when that vote was cast. This might also create a "Election Imminent" event.
- When the countdown ends, the core calls its own elect function.

So the core needs to be thread safe?
Or at least its mutating functions need to be thread safe.

pub struct Core<PID: ID, GID: ID> {
    id: GID,
    state: State<PID>,
    rules: Rules,
    stats: Stats<PID>,
    event_out: EventOutput<PID>,
}

id, rules, are immutable

state and stats are mutable

event_out is a mpsc queue

state and stats need to be protected. Maybe combine them into a single struct.
That way a state mutex can be used to protect both.
Maybe we do, then want these functions to operate on the state, not the core...
And we can pass in rules as they apply, and pass in the event queue.

This then means that we can start an elect timer. 
Upon the end of the elect timer, we call elect on a state.
A clone of Arc<Mutex<State>> is passed to the elect function.

*/

pub struct Timer<T>
where
    T: 'static + Send + Clone + Debug,
{
    end_time: Arc<Mutex<SystemTime>>,
    cancelled: Arc<Mutex<bool>>,
    data: T,
}

impl<T> Timer<T>
where
    T: 'static + Send + Clone + Debug,
{
    pub fn new(
        end_time: SystemTime,
        data: T,
        precision: Duration,
        callback: Box<dyn FnOnce(T) + Send + 'static>,
    ) -> Timer<T> {
        let cancelled = Arc::new(Mutex::new(false));
        let end_time = Arc::new(Mutex::new(end_time));
        let timer = Timer {
            end_time: Arc::clone(&end_time),
            cancelled: Arc::clone(&cancelled),
            data: data.clone(),
        };
        thread::spawn(move || {
            let mut now = SystemTime::now();
            while now < *end_time.lock().unwrap() {
                thread::sleep(precision);
                now = SystemTime::now();
                if *cancelled.lock().unwrap() {
                    return;
                }
            }
            callback(data);
        });
        timer
    }

    pub fn cancel(&self) {
        *self.cancelled.lock().unwrap() = true;
    }

    pub fn set_end_time(&self, end_time: SystemTime) {
        *self.end_time.lock().unwrap() = end_time;
    }

    pub fn get_end_time(&self) -> SystemTime {
        *self.end_time.lock().unwrap()
    }

    pub fn get_data(&self) -> T {
        self.data.clone()
    }
}

impl<T> Debug for Timer<T>
where
    T: 'static + Send + Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timer")
            .field("end_time", &self.end_time)
            .field("cancelled", &self.cancelled)
            .field("data", &self.data)
            .finish()
    }
}

impl<T> Drop for Timer<T>
where
    T: 'static + Send + Clone + Debug,
{
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer() {
        println!("Testing timer");
        let now = SystemTime::now();
        let end_time = now + Duration::from_millis(3500);
        let timer = Timer::new(
            end_time,
            5,
            Duration::from_millis(100),
            Box::new(|n: u32| println!("{:?} Timer expired", n.clone())),
        );
        thread::sleep(Duration::from_secs(1));
        println!("1");
        let et = timer.get_end_time();
        println!("{:?}", et);
        timer.set_end_time(et + Duration::from_secs(0)); // now set to 3
        println!("test");

        thread::sleep(Duration::from_secs(1));
        println!("2");
        thread::sleep(Duration::from_secs(1));
        println!("3");
        thread::sleep(Duration::from_secs(1));
        println!("4");
    }
}
