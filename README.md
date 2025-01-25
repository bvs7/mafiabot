Mafia Bot is a rust program that moderates games of Mafia over ~~Discord~~ Groupme.




# Actors and Channels

## Actors

### API Handler
Handles all requests sent to the Groupme API.
All this does is it has a task that runs on a timeout interval. It calls `notify_one` on a `Notify` item on that interval
Then, all we have to do is have each call to the handler block on that `Notify` for each call.
This means, for most messages, we can just spawn a task and detach it.

### Game Actor
Loops on ActionRx (and timeout)
Calls the Action on the State, spawns a task to respond via api
Updates state to StateWatch
And also checks timers and updates timeout
Also updates known names from Main Chat

### Event Listener
Awaits EventRx from a specific Game, and converts events into API calls
It can just spawn a new task for each call.

### LobbyRx
Receives Lobby Commands from a LobbyCmdRx (and timeout)
With each command, updates state (of start message or other).
Has access to its StatusWatches for games, so can answer status queries
Sends responses to api usually via a new task.
Except for creating start message, which awaits response.
It could just store the future with the timeout, and only await it when the timer finishes...

Although we do want to know which games are ours

How to start new games? It can just spawn it itself, and everyone will notice the start message

### CmdParser
Receives PushMessages from Websocket
And also receives Events to know about new Lobbies or Games.

In fact, receiving start events seems like a pretty general thing

## Channels

### EventBroadcast
Might change the name of this. Used for...
- Game Events with payload to produce output messages

### ActorUpdateBroadcast
Sends Create and Destroy messages for different Actors (Games and Lobbies)

### ActorWatches
`WatchRx` receivers for Games or Lobbies, where they update their state regularly.

### GameCmdMpsc
- Game Action Command


# TODO

Pack up mafia into its own crate

Create the GameActor

Create the LobbyActor

Let's think about dependency hubs
