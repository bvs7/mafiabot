
# Mafia Game Design

## Structure

At the core, we have the core crate. This implements the game of Mafia itself, and some of the info surrounding it.

Importantly, we have to be able to schedule things. Right now the Game struct is in an `Arc<Mutex<Game>>`. This implements a kind of monitor behavior. We can then also pass the monitor reference down to the lower levels of processing, and clone it into closures, etc.

"Actions" are inputs sent to the Core. The Core returns "Events", or potentially Errors

Hmmm Actions shouldn't be sent through a channel, separately, they should be a direct input. That way the Error can return directly.

GameWrapper is a struct with the `Arc<Mutex<Game>>` field. You can call an action on it, and it will pass that action into the Game, along with a reference to the `Arc<Mutex<Game>>`. This way, the Game can schedule an event in the future, if needed.........

This seems lopsided. I don't like needing to pass that in so far.

Another idea is that Game.handle_action() returns the appropriate info needed to schedule something. For voting, that is a Choice to check later. For targeting that is just whether all targets have been placed...

Also, an "End Day" coming from a timer would need to, check election, if no election, elect Abstain. Similarly, a End Nighe from a time should be fine, assuming targets that don't exist can be tolerated.

### Scheduled Checks

Given actions:
- Vote
- Reveal
- Target
- Mark

What do these actions need to return to schedule a check?

- Vote: Choice elected?
- Reveal: N/A
- Target | Mark: bool "All targets chosen"

What does the caller expect in response? Either an error or () is fine.

## Elections
We want to keep track of who is the hammer in elections where a player is elected.

So, a new enum? Election::Hammer(PID, PID) | Election::Abstain


## Serialization

Have Ser_ Versions of data structures, and a way to convert from one to another.

Validity checking

Game Data vs Data needed after game?
Role History can do it, I think.

GUARD (1) -> AGENT (2). then if 2 is alive at end of the game, win

IDIOT -> Elected IDIOT!

Game Phase::End has winning team info, survivor info

Names... Optional? Useful for Serialization, makes things more readable...


What data do we need for a game???

Players -> Alive? + RoleHist

- During game
    - Phase data
    - Players -> Current Role
    - Game Rules
- After game
    - Winner?
    - Role Assignments/History
    - Whether IDIOTs won?
        - Have list of Events? Check if Idiot won there?
```rust

pub struct RoleHist{
    role: Role,
    history: Vec<Role>,
}
pub struct Players(HashMap<PID, RoleHist>);
pub struct Names(HashMap<PID, String>);

pub struct Game {
    id: u64,
    players: Players,
    phase: Phase,
    rules: Rules,
    names: Option<Names>,
    eo: EventOutput,
}
```

```rust
pub enum SerRole{
    Role(Role),
    Hist(Vec<Role>),
}
pub struct SerPlayers(HashMap<String, SerRole>);
```

```rust
Players + Names -> SerPlayers
    PID -> String, RoleHist -> SerRole

/*
{players: {
    "Name 1": "TOWN",
    "Name 2": "MAFIA",
    "Name 3": {"IDIOT":false},
    "Name 4": {}
    }
}
*/
```