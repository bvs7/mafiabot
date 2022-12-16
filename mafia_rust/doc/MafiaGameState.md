

# MafiaGame

Holds data for a mafia game...

## Interfaces

### Input Actions

A channel sends actions (vote, target[, timer]) from somewhere.

Command Behavioral pattern:
- `Command` is an impl. `Vote` and `Target` are structs that change Game State Accordingly?


### Output Info Events

Events are output.

Possible Event Handlers:
- Moderator (sends game update messages, updates game status, etc)
- Recorder (stores data on what goes on in games for future review)
- Saver (stores game data for recovery later)

These event handlers should be implemented modularly, so that the game state is agnostic to them?

Possible events:
- Vote
- Elect
- End Day?
- (Start Dusk)
- (Vengeance)
- (End Dusk)
- Start Night
- Target
- End Night
- Start Day

## State

### Helper Types

- Field<T> : HashMap<&Player, T>

- enum Role
- User(String)
- enum Contract (can contain data)
- enum Selection<>
- struct Day
    - n: u32
    - votes: Field<Selection>
- struct Night
    - n: u32
    - targets: Field<Selection>
 
- players: Vec<Player>.           Owned instance of all players that start the game. Could be replaced by p_id: Eq + Hash
- roles: Field<Role>.    Player mapping to role...
- alive: Field<bool>.          Contains living players (kind of like a bool in player?, but modifiable)
- users: Field<User>.    Player mapping to user
- contracts: Field<Contract>
- enum phase:
    - Init
    - Day (Day)
    - Night(Night)

## Game progression

- Game is created with all starting info.

## Maybe no cute data structure?

What needs to be known?
- Players playing
- Roles
- Users
- Who is alive

- Player includes Generic user class. That class can be used in HashMap.
- roles are a Field and can have contracts
- alive is a Field
- phase
    - votes are a Field
    - targets are a Field...

- Validated Fields have a check command. Check that it is valid over every player.


- Or, when you enter a player you must enter the other things as well?

## Unique UID idea

Player has uid field which is unique.
Player struct has relevant data.
Votes and Actions still in phase.

When night ends...

actions : HashMap<U, Target<U>>
let actions: Vec<U, Target<U>> = actions.iter().collect()