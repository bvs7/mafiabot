
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


## Controller

Idea. Split controller functionality up.

- Controller
    - `lobby`
    - `GameState`
    - `rules`

- Lobby. Starts games.
    - Handle fn signature: `lobby.handle(cmd: LobbyCommand, game_state: &mut GameState)`
    - Commands
        - `/init` - Creates (or clears) main and mafia channels.
        - `/join` - Adds someone to the (not started) game, or add someone to a started game
        - `/leave` - Removes someone from a not started game, or removes a dead person from a game, or removes someone from an ended game
        - `/start` - Starts the game
        - `/close` - Removes the channels of an ended game
    - Data needed?
        - `lobby` - info about lobby that might start game
        - `GameState` - enum of state of game.
            - `None`
            - `Init`
                - `users: Vec<UserID>` - List of users that will join game
                - `GameChannels` - Main and mafia channels
            - `Play`
                - `Game`
                - `GameChannels`
                - `Receiver<Event<UserID>>`
- GamePlayer. Handles a game in progress.
    - Handle fn signature: `player.handle(cmd: PlayCommand, game_state: &mut GameState)`
    - Commands
        - `/vote`, `/reveal`, `/target`, `/mark`
        - `/timer` (eventually...)
    - Data
        - `Game`
        - `GameChannels`
        - `Receiver<Event<UserID>>`
        - `players` for Event responses?? (Or maybe this is inside eventhandler)
- Help. Handles help messages
    - `/help [subject]`
- Rules Handles rule changes
    - `/rule [rule] [setting]` (Or dropdown menu?)
    - `/role [command] [role]` (Or checkbox menu?)


# Refactor

## Control Flow

- A vote action is passed to the Game.
- The Game processes the vote and notices an election is ready
- The Game schedules an election check in X amount of time
- The election check passes
    - Election occurs
    - Update roles with contract details
    - Eliminate Player
    - Update Phase

### Structure
So, because we need to be able to schedule an election check in a separate thread, and we want that thread to be able to touch everything about a Game, we will put the game in a mutex.

Do Tokio later. For now...

- **Game Wrapper**,
    - **Mutex\<Game\>**
    - Action queue
    - Event queue
    - Methods
        - run
            - main threads
            - One waits on action queue and pipes it into Game
            - The other waits on event queue and ...
            

Struct/Enum hierarchy?

- **Procedure** the context and routing of a game?
    - **Mutex\<Game\>** all of the info about a game?
        - *State*
            - Play
                - *Phase*
                    - Day
                        - votes
            - players
            - roles

Idea: Put all code in GameStruct file.

No impls? Just straight functions in a namespace? SO this is the folder that processes the game? Process?

Monitor style obtain Game Mutex

Overdesigned right now?

State -> Init | Play | End

State is the dynamic info necessary to the game right now. It should be possible to fully simulate everything through Play.

Extra important info is maintained in the GameContext

Note that `Role`` is `Role_<PID>`

roles contains all role mappings, and those roles are never removed, but might be changed. This is how contracts are managed?

How is Idiot win state recorded?

When the IDIOT is elected... They die and they win. Idiot has win state

What do we need for spawning an election thread? Access to the GameContext

So, we want the action thread running in GameContext to be able to spawn an end of phase checker? That has a time delay, hence spawning.

Action Receiver might be a tuple with response context?

Response context needs to know... how to route normal game messages?

So if our stack looked like...
- GameContext.run()
    - game.lock().unwrap()
    - handles ActionErrors? Routes response...
- game.handle(action, lock, event_output) -> `Result<bool,ActionError>`
- state.handle(action, lock, event_output) -> `Result<bool,ActionError>`
    - check phase ?
    - check PIDs ?
    - edit Day.votes
    - check votes, and spawn thread on 
    

Uh Oh.
What if we schedule an election, and in the meantime, the Game ends?
The point is, we can't put the lock in a closure as it is. Maybe use an Arc or smth?

hmmmmmmmm... It doesn't quite make sense to pass a reference to yourself inwards... we need to find another way. Potentially all code is handled in GameContext level?

```rust

pub struct GameContext {
    game: Arc<Mutex<Game>>,
    actions: Receiver<Action>,
}

pub struct Game{
    game_id: usize,
    state: GameState,
    rules: GameRules,
    role_history: RoleHistory,
    event_output: EventOutput (Sender<Event>),
}

pub struct GameState {
    day: usize,
    players: Players (HashSet<PID>),
    roles: HashMap<PID, Role>,
    phase: Phase,
}

pub enum Phase {
    Day {
        votes: HashMap<PID, Choice>,
        blocks: HashSet<PID>,
    },
    Night {
        targets: HashMap<PID, Choice>,
        scheme: Option<(PID, Choice)>,
    },
    Dusk {
        avenger: PID,
        voters: HashSet<PID>,
    },
    End {
        winner: Option<Team>,
    }
}
```

At end... You can tell which Rogue roles won:
- Agent | Guard | Survivor: based on who is alive
- Idiot: based on Role enum bool

Allowing Role values allows things like one-shot vigilantes or milkmen, etc.

