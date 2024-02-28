
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