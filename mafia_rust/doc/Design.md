
# Mafiabot Design

Mafiabot is a discord chatbot that facilitates a game of Mafia.

## Gameplay Overview

Mafia is a hidden information game between two teams. The "Mafia" and the "Town". The Town are trying to discover who the Mafia are and kill them, while the Mafia know who each other are, and are trying to kill town.

The game commences in Phases, alternately Day and Night. During a Day phase, all players may publicly cast a single transferrable vote for other players. Once a player has received a majority of votes, they are elected and eliminated and the game proceeds to the Night phase. During a Night phase, the Mafia will target somebody to be killed. At the end of Night, the targeted player will be eliminated.

Aside from the Mafia aligned normal MAFIA role and the Town aligned normal TOWN role, there are other special roles, such as COP, DOCTOR, CELEB, GODFATHER, etc. Some roles also have night actions, such as investigating or saving other players. All night actions are selected during the night, then resolved at the end of Night, which happens once all selections have been made. Some roles are also not aligned with either Town or Mafia. These Rogue roles have specific goals, such as getting voted out, protecting another player, or causing another player's death.

## Gameplay Experience

Currently unimplemented, the plan is to use Discord to facilitate the communication and game mechanics.

When a game is created, two channels will be created for the game. A "Main" channel which includes all players, and a "Mafia" channel which includes only mafia members. Using discord permissions.

Additionally, a separate channel is created for each player with a role that has a night action (COP, DOCTOR, and STRIPPER)

The bot sends messages as the Moderator

At the start of the Day phase, Moderator announces the day
- Moderator: `Day #2. 7 players alive, 4 votes needed to elect`

During the Day phase of the game players can send various voting commands to direct their vote accordingly:
- Alice sends `!vote @Bob`
- Moderator responds `Alice votes for Bob, 2/3 votes to elect Bob`
- Alice sends `!unvote`
- Moderator response `Alice retracts vote`
- Alice sends `!vote nokill`
- Moderator responds `Alice votes for peace, 1/2 votes for no election`

When enough votes are garnered, Moderator announces the result
- Moderator: `Charlie votes for Bob, 3/3 votes to elect Bob`
- Moderator: `Bob has been elected to die`
- Moderator: `Bob was Town aligned`

At the start of Night phase, Moderator sends out options to the Mafia channel, and to night role channels:
```
Choose someone to kill:
A: Alice
B: Charlie
C: David
D: Ellie
E: No Target
```

Players can send their target during the night phase:
- Alice: `!target D`
- Moderator: `You have targeted Ellie`

Once all night actions have been completed, night is resolved. This involves various things, such as STRIPPERs blocking other targeting roles, COPs investigating, DOCTORs saving, and the Mafia targeting a player.
- Moderator to Alice: `Ellie is Town Aligned`
- Moderator to Main Channel: `Charlie has been killed!`
- Moderator to Main Channel: `Day #4, 3 players alive, 2 votes needed to elect`

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
            - targets `HashMap<Pidx, Target>`
            - scheme `Option<Pidx, Choice<Pidx>>`
    - Contracts - List of Rogue roles, goals, and victory status

### Contracts

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

## Rolegen

### Spice Rolegen

Provide a "spice" level from 0.0 to 1.0. 

Pick number of mafia/rogue in the usual way.

Assign Rogue roles according to game scoring

Pick a number of spicy town and mafia roles.

Randomly pull spicy town and mafia roles.