
See doc/Design.md for gameplay goals

## TODO
- Core
    - ~~Contracts/Rogue victories~~
    - Add new features
    - Saving (Design)
        - Format (files? databases?)
        - Frequency
        - Recovery/Loading
    - Testing
- Controller
    - Starting a Game
        - `!init` and `!start` message handling
        - Rolegen
        - Creating channels and editing permissions
    - Passing Commands to Game
        - InvalidActionError handling
    - Handling Core Events and outputting messages
    - Testing
    - *Saving and Loading Games*
- Server
    - Sending Messages
    - Creating Channels (and groups)
    - Permissions Management
    - Command Parsing

# Architecture

- Core. The core logic of the mafia game
- Controller. Handles Lobby commands, and passes Game commands to the Game
- Server. Receives Discord Bot API callback requests and routes them to a controller.

## Core
The core logic of the mafia game is encapsulated in the core module.

The core can be interacted with by calling the `handle(Action)` method of the `Game` struct. This returns a `Result` of either `Ok` or an `Error`, if the Action was invalid. The `InvalidActionError` explains why the Action was invalid.

After any call to `handle()`, the core may generate Events and push them to a thread safe queue. Events are how the core outputs information to the players.

This general form of encapsulation is meant to allow the Core to use different mediums for gameplay. Mafiabot has previously used GroupMe, but Discord currently seems like the best option.

### Core Inputs: **Actions**
Actions are things players can do to change the state of the mafia game. Currently, the available actions are:
- **Vote**. A vote cast by a Player during the day
- **Reveal**. A CELEB's ability to prove their role during the day
- **Target**. A COP, DOCTOR, or STRIPPER selecting the target of their night action
- **Mark**. A Mafia Aligned Player selecting the Player that will be killed in the night

Potential Future Actions include:
- **EndPhase**. An immediate end to the current phase (Usually because a time limit was reached)
- **Guess**. A way for a Player to guess the role of another player (See the WITCH role idea)

#### **Vote**

Voting can only occur during a Day phase and is how the players colelctively determine if and who will be "elected" to die (some versions of mafia refer to this as lynching, but I use the term elect). Players can vote for:
- Another Player
- No Election
- None (Vote Retraction)

Therefore, the data associated with a Vote is:
- `voter: UserID` (Player who votes)
- `ballot: Option<Choice<UserID>>` (What is being voted for)

Where `Choice<U>` is an enum of either `Player(U)` or `Abstain`

Handling this Vote Action updates the player's vote publicly, and possibly results in an election.

#### **Reveal**

"Revealing" is an Action a player with a CELEB role can take. It can only occur during a Day phase. It involves sending a `!reveal` message to the Moderator in a private channel. If the CELEB is able to reveal, the Moderator will send a message to the Main Channel saying "[player] is CELEB", proving that the player is Town Aligned. The only data associated with a Reveal is the `UserID` of the CELEB revealing.

#### **Target**

Targeting occurs at night and is done by each role with a night action (COP, DOCTOR, STRIPPER). These roles target someone, and once all night actions have been completed (including the Mafia's Mark seen below), the end of the night will be resolved, taking all night actions into account.

Data associated with a Target:
- `actor: UserID` (Player with Night Action who is targeting)
- `target: Choice<UserID>`

Note that there is no way to retract a Target, unlike Votes

#### **Mark**

Each night, the Mafia choose a mark to kill. This is done by a Mafia Aligned player in the Mafia Channel selecting a mark. The mafia player who selects the mark is the killer. Which specific Mafia member is the killer is important.

Data associated with a Mark:
- `killer: UserID` (Mafia member who selects target)
- `mark: Choice<UserID>` (Who to kill, or `Abstain` to kill nobody)

### Core Outputs: **Events**

Events are generated and added to a queue as the core handles different Actions.

Events will usually generate a message in one or more game channel.

Events:
- Start
- Day
- Vote
- Retract
- Reveal
- Election
- Night
- Target
- Mark
- Dawn
- Strip
- Block
- Save
- Investigate
- Kill
- NoKill
- Eliminate
- Refocus
- End

## Controller

Currently unimplemented, the Controller handles all of the bot operation that is not game logic. It implements "Lobby" commands, where players can create and start a game, request game stats, etc. It routes Game Actions into Game Cores, and handles error responses for invalid Actions. Once timers are implemented, it spawns the timers/alarms when requested.

### Lobbies

Users are collectively part of a channel called a Lobby, where games can be created.

In the past, the method of starting a game had a User send a `!start` command. They could also specify a minimum number of players, and the number of minutes after which the game would start. Users could theh like the message to try to join the game. A game should be able to be started with any number of players of at least 3. After the amount of minues specified, the game start would be attempted.

#### Start

I foresee a similar method being used in discord, with potentially more control. When a user sends a `!start` (Or maybe `!init`) command, the Moderator sends a message that says "React to this to join a game". Users can then react to that message to attempt to join the game. There can be unique reactions that specify intents as well.
- *️⃣ Could be used to join a game with any number of players
- 3️⃣ 4️⃣ 5️⃣ 6️⃣ 7️⃣ 8️⃣ 9️⃣ 🔟 Could be used to join a game with at least that number of players?
- ▶ Could be used to specify that players were ready to begin?
- Other Emoji could specify other preferences?

A second command (`!start`) could be used to try to launch the game. The following things happen when a game is started.

- The Users who want to play are collected, along with their conditions for playing
- If valid game cannot be created, end.
- Rolegen. Create the roles for the game and assign them to users creating players.
- Create the appropriate channels for the game and add the players to them.
    - Potentially assign Discord Roles (Heretofore referred as "Player States")
- Add the appropriate permissions for players to see the proper channels.
- Create the Game Core using the Player list.
    - Potentially spawn a thread to handle the Game Core's Event queue.
- Start the Game Core

#### Create

Another way to create a game might involve more control on the part of one user. A user with appropriate permissions might create a game by mentioning all users that want to play in a `!create` command.

A potential feature would be letting the user set up the rules and roles of the game privately before or while creating it.

#### Join

In previous iterations of Mafia Bot, there was no way to add a player to a game. However, I think there might be a way to allow it seldomly. The proposal might be to add a Player as a random Rogue role, so as not to unbalance the game.

### Results

When a game finishes, the Controller will need to...
- Clean up the Game Core object.
- Probably announce the results to the Lobby
- Return Users to the appropriate Discord Roles.

### Minimum Viable Product

The requirements for the MVP of the controller are:
- Has ONE channel as the lobby
- Can create ONE game at a time via a very simple `!init` -> `!start` command system, where reacting to the `!init` response with *️⃣ lets you join.
- Does not touch Discord Roles for now.
- Must be able to create channels for games.
- Parses commands from server into Lobby Commands or Game Actions
    - `!init`
    - `!start`
    - `!vote`
    - `!unvote` or `!retract`
    - `!target` or `!mark`
    - `!reveal`
    - `!status` (Temporary command until status of game can always be displayed by editing a message?)
- Handles Invalid Game Action Errors and responds to sender appropriately.
- Cannot set Rules or Roles for games. Merely uses a standard set

## Server

The Server is what receives Discord Bot API callback HTTP requests. It identifies game commands and routes the relevant data to a Controller.

### Minimum Requirements

- Identifies commands and gets appropriate data, (UserID of sender, ChannelID of channel)
    - `!init`
    - `!start`
    - `!vote` (@mentions and following word e.g. `!vote nokill`)
    - `!unvote` or `!retract`
    - `!target` or `!mark` (following work e.g. `!target A`)
    - `!reveal`
    - `!status`
- Sends these commands to Controller.
- Allows responses to be sent to specific Discord Channels
- Allows requests to get the reactions to a specific message
- Allows creating channels and changing permissions for those channels

