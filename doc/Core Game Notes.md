
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