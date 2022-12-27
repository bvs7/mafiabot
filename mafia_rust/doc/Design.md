
# Mafiabot Design

Mafiabot is a discord chatbot that facilitates a game of Mafia.

## Gameplay Overview

Mafia is a hidden information game between typically two teams. The "Mafia" and the "Town". The Town are trying to discover who the Mafia are and kill them, while the Mafia know who each other are, and are trying to kill town.

The game commences in Phases, typically alternately Day and Night. During a Day phase, all players may publicly vote for other players. Once a player has received a majority of votes, they are elected and eliminated and the game proceeds to the Night phase. During a Night phase, the Mafia will target somebody to be killed. At the end of Night, the targeted player will be eliminated.

Aside from the Mafia aligned normal MAFIA role and the Town aligned normal TOWN role, there are other special roles, such as COP, DOCTOR, CELEB, GODFATHER, etc. Some roles also have night actions, such as investigating or saving other players. All night actions are selected during the night, then resolved at the end of Night, which happens once all selections have been made. Some roles are also not aligned with either Town or Mafia. These Rogue roles have specific goals, such as getting voted out, protecting another player, or causing another player's death.

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

#### Contracts

Semantics
- Contracts given at beginning of game
- Type can be `Assassinate` `Protect` `Elect`
- When that is met, (election or elimination)
    - Search contracts for a charge that died/elected
    - Note the outcome on contract?
    - Potentially refocus...

Store in player? Or store in a separate field?
Need separate field, mut retain info after player deaths

How to handle contract wins?

Must announce win/lose for each of them at the end of the game, so do we even need to declare some special win?
