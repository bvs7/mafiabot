
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

We have a Core object.

You can call various methods of that Core object to change its state and send out events.

Some of these events might spawn a timer that will do something later.

Specifically, an election and a dawn can be scheduled to happen
- The election so that players have a chance to unvote (within 10 seconds or so)
- the dawn so that players don't know if they are the only or the last night action to occur.

If we create another thread, what needs to be shared?
- Event_Output can be cloned into the new thread.
- the shared Arc<Mutex<State>> can be shared.

State mutating functions... should be performed on state.
Pass in whatever else is needed, but fundamentally...

Stats should probably be part of state.

State mutating functions:
- vote
- target
- scheme
- to_day
- to_night
- elect
- eliminate
- dawn

However, these functions all also require other parts of core:
- the event output Sender.
- A copy of the rules.
    - How could accessing the rules work? Normally they would not be updated, but we should probably be able to update them if necessary.
    - This would mean that on every call, the rules might be updated? How are they updated?
    - This makes me think we maybe should have some kind of process running for the Core Game...
    