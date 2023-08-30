## Discord Features?

- Channels
    - Categories
    - Threads
- View/Send Message Permissions
- Message Components!
    - Buttons for joining games?
    - Menus for selecting a target!
- Application Commands
    - Text Commands (Standard easiest voting messages)
    - Message Commands
    - User Commands (Voting and Targeting?!)
- Roles
    - Indicate who is In a game, alive, dead, or spectating, but only one per server...

### Threads

Use threads for specific chats?

Have one main game channel, then have a mafia thread, and a thread for each player?

Or potentially just use main and mafia channels, then have threads to reveal roles?

## High Level Overview

How does a game happen?

- MafiaBot is added to a server

- A server member requests that a game starts
    - MafiaBot creates a join button
    - MafiaBot creates a new channel for the game
- The game starts
    - Roles are assigned via threads
    - Mafia are given a thread
- The game is played
    - Handle Game Events
- The game ends
    - The channel is ready to be deleted? Schedule that to happen?

So it seems like it would be good to have an event that is "game initialization". This doesn't include the game being started, but simply creates all of the necessary infrastructure for joining the game.

Should we expand the core functionality? Create a game, but no players. This allows for an init event that will create the channel and join structures.

### Core Event Handling

- Init
    - Create game channel
    - Create join button and leave button and start button
    - Create watch button?
- Start
    - Remove join button
    - Send out roles
    - Send out start info
    - Set Discord Roles
- Day
    - Send day start message
    - Allow day chatting
- Vote
    - Send confirmation
- Retract
    - Send confirmation
- Reveal
    - Send info to channel
- Election
    - Send info to channel?
- Night
    - Send night start message
    - Give selection menu for targeters
    - Allow night chatting, stop day chatting
- Target
    - Send confirmation
- Mark
    - Send confirmation
- Dawn
    - Send udpate, stop night chatting
- Strip
    - Send info to stripper
- Block
    - Send info to blocked
- Save
    - Send update based on rules?
- Investigate
    - Send info
- Kill
    - Send message
- NoKill
    - Send message
- Eliminate
    - Send message
    - Reveal based on rules
    - Change Discord Role
- Refocus
    - Send message with update
- End
    - Make close button

### Discord input handling

- Join Button Pressed
- Leave Button Pressed
- Watch Button Pressed
- Vote Command Recvd
- Target Command Recvd
- Mark Command Recvd
- Reveal Command Recvd
