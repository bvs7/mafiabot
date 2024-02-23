
Core should use generics for:

Game_ID
Player_ID

Use channel id for game, and user ids for players

`Core:`
- `id`: Unique game id
- `State`: Current game data
- `Rules`: Rule set the game was started with. Used for computation
- `Stats?`: Information about how the game has gone

Core State enum:
- `Play`
	- `day_no`
	- `players: HashMap<PID, Role>`
	- `phase: Phase`
- `End`
	- `winner?`

## Phase
An enum with the following values:
- Day
- Night
- Dusk

### Day
- `votes: HashMap<PID, Choice>`
- `blocks: HashSet<PID>`
### Night
- `targets: HashMap<PID, Choice>`
- `scheme: Option<(PID,Choice)>`


Functions:
- Vote
	- Change `votes`
	- Calculate election results? -> Election Result
	- If Election Result is an election:
		- Eliminate someone
		- Change `phase` to `Night`
- Target
	- Change `targets` or `mafia_target`?
	- Check for dawn.
	- If Dawn:
		- Record Blocks
		- Record Saves
		- Perform Scheme
			- Eliminate someone
		- Perform Investigations
		- Change `phase` to `Day`
- Reveal
	- Check for blocks
	- Reveal


