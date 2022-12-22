
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
        - comm::Event structs
        - Errors returned from handle
        - access mutex and check Phase status