
# Mafiabot Design

Mafiabot is a discord chatbot that facilitates a game of Mafia.

## Architecture

- Server: Interfaces with Discord
- Controller: Sets up and handles Games
- Core: Runs game logic


## Ideas
Status accessibility.

We need to read game data regularly to update users as to the status of a game. It might also be good to know what users are in the game at some point?

Core Game is behind a mutex. It must be grabbed to do any game logic.

Right now, Comm has a rx channel and a tx channel. Remove the rx channel.

Game has command passed in. We used to want "Source" to know where to return info...

Events should be standalone. If something requires context to be returned (i.e. Invalid Command response), that is an Err return from game.handle(). So the caller needs the context to resolve that.