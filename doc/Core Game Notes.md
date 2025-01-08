
## 2025 refactor:

Should the RwLock be internal? Yes. So State should have everything, right?

Our interface consists of:
- external `action_tx: mpsc::Sender<(Action, oneshot::Sender<Result<...>>)>`
- external `event_rx: broadcast::Receiver<Event>`
- `pub game_id: u64` field
- `pub state: State` field
- `pub rules: Rules` field
## Returning 2025 ideas:
Axum. Make the core an API
- Figure out how to create a new core (it will spawn its own app/server)
- Header contains user key?
- Endpoints:
  - GET / -> get game status
  - GET /events -> get events from a game
    - after? -> after a specific event_id
  - POST / -> send an action to be processed

The core will be a state with...
- An action queue
- Game State
- An event log

The status can be pulled at any time? Cloned and put into the response...
Action queue entries are an action and a oneshot response. The response is formatted into the axum response
One idea is that the action queue is actually a command queue, and a command is either an action or a status request...
	Need to think about this part. 

State needs to be Clone + Sync + Send
This includes tokio::sync::mpsc::Sender... (Action Queue)
Otherwise... we want to be able to grab Game State and Event Log...
One option is include event log in greater game state. Put Game State in an `Arc<Mutex>`? Seems easy enough
No, wait, use RwLock. All gets can read simultaneously, the action queue can write

RwLock does have a queue... So we could just use it as the queue...
But I think I prefer a mpsc channel. That leaves a separate task to just calculate game actions. This task will wait on the queue,
Pop off an action/return oneshot channel, pick up the RwLock as a write, then execute the action.

How to think about things like elections? Maybe have some kind of state for impending actions. Have a destination time for that action.

So possibly always have a task that waits for a timer to be scheduled, polls that timer, and executes the action? Is it an action? Should it be queued? No, it isn't an action, but it grabs the RwLock to write.

Implement timers by spawning a tokio task, and holding the joinHandle... If that task can be stopped, abort it.

We want to be able to abort the election if another vote comes in. We want the election task to grab the lock, not just submit an action.

Putting actions into the channel allows passing to another task. This lets the passing task return quicker, as soon as it is validated.

Or, have the main loop select between timer return and action queue!!!
pin the timer return though, please

So what if we have multiple timers? Do we want to be waiting on all of them? There could also be some kind of notify that joins the election. Yeah, then we wait for the election to finish. Makes sense.

So now, we create a server on the tokio runtime, as well as a game handler task.
Requests either:
- take the Game state mutex, to return status or events
- Push an action to the action queue, then wait for the response of running that action.



## Library Privacy

How should the Core Game stuff be organized?

- The Core will be split into multiple files for organization reasons
- The Core itself includes mostly game logic, including basic accessor functions like new() and start()
- Interface has the structs for how a controller interacts with the Core Game object
- There are some very basic structs that should be easily available
	- Roles
	- Teams
	- "ID" trait
	- Choice
- Rules should probably be its own

So right now we have:
- mod.rs
- base.rs
	- PID: ID
	- Choice
- role.rs
	- Roles
		- Role implementations for night actions?
	- Teams
- interface.rs (Action and event)
	- error.rs
- timer.rs
- state.rs
	- stats.rs
- rule.rs

For now make it all public

## Tokio and concurrency

How should we allow the core to be used by a tokio runtime?

The run function of the core should be async?

```rust
async fn action<T>(&mut self, action: Action<PID>) -> Result<T,CoreError<PID>> {

}

```

Where can the errors be in this process?

- action() (Action, RespInput) -> Core
	- ActionSend
	- **ActionRecv**
- Core (Resp) -> action()
	- **RespSend**
	- RespRecv
- Event -> send


Idea: just append events to a queue, then push those to the event channel later?? That way we don't need everything to be async?

Having to await various things in the middle of an operation seems bad...

Look into redoing timer:
- Timer is a task that is waiting for end time.
- It is also waiting for end time to be changed. Either of these things should wake it up?

Idea: Don't use SystemTime for anything except storage?
- When saving a timer, get current instant and current System time, and add time remaining to system time