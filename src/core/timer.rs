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
