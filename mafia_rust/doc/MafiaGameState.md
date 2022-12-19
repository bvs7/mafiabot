

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
