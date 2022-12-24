
# Mafiabot Design

Mafiabot is a discord chatbot that facilitates a game of Mafia.

## Game Design

- "Block"
    - Happens behind the scenes
    - Night clears block list
    - Night block list carries into day
- "Stun"
    - Notified at start of night, when options are listed
    - Stun list, any Night Targets (or Marks), can only Abstain

- IDIOT!Stun imparts stun at beginning of Night to electors
- GOON always stuns self at beginning of Night
- DOCTOR!Stun...
    - Need a list for stunned doc? Or edit Role?
    - DOCTOR!Stun saves self => DOCTOR!Stunned gets stun at start of night => DOCTOR!Stun

## Architecture

- Server: Interfaces with Discord
- Controller: Sets up and handles Games
- Core: Runs game logic

### Interfaces

- Core
    - Inputs are comm::Command structs
    - Outputs
        - comm::Event structs for game outputs that don't need context to be routed to users
        - Errors returned from handle need context (Who sent wrong command and from where?) To respond
        - access mutex and check Phase status done by exterior thread to update game status

### Module Structure

- Core: Facilitates the game logic.
    - Functionality is all in the `Game` struct
    - Type system for data.
        - Publically accessible data?
        - Vs internal
    - Interface (Commands and Events)

- Core Data:
    - Players* Constructed from outside? So must be public?
        - UID
        - Role
        - ~~Name~~
    - Phase
        - Init: Pre Start?
        - Day:
            - day_no
            - votes `Vec<Pidx, Choice<Pidx>>`
            - blocks
        - Night:
            - night_no
            - strips `Vec<Pidx, Choice<Pidx>>`
            - saves `Vec<Pidx, Choice<Pidx>>`
            - investigations `Vec<Pidx, Choice<Pidx>>`
            - kill `Option<Pidx, Choice<Pidx>>`
        - Dawn?:
            - dawn_no
            - block_map `HashMap<Pidx, Vec<Pidx>>` // blocked -> [strippers]
            - save_map `HashMap<Pidx, Vec<Pidx>>` // saved -> [doctors]
            - investigations `Vec<Pidx, Pidx>`
            - kill `Option<Pidx, Choice<Pidx>>`

#### Targeting system

- handle
    - handle_vote
    - handle_retract
    - handle_reveal
    - handle_target
    - handle_mark

- Command Validation
    - Return errors if command is invalid
    - return translated command data
- Command acceptance
    - Passed to a specific phase struct, as this is where the data needs to be stored.
    - Returns a result showing if the phase would end, and how

- Combine acceptance and resolution?