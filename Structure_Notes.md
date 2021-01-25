# Structure Notes

How should the structure of classes, inheritance, dynamic class type attributes, etc be structured?

## Game Logic/Utility Separation

We want to be able to separate the Game Logic (Mafia) from the platform implementation (GroupMe/Discord/etc.). Ideally the Game Logic can be changed independent of the platform, and vice versa. Theoretically, it should be possible to make an entirely different game using the same platform classes?

### MChat and MDM

This involves a Chat and DM class that can be extended by a platform specific class. This provides and interface for communicating with these entities.

Currently the MChat interface includes:
- __init__(group_id, name_reference=): Create a handle to this chat from this id, when placing in names for Player IDs, use the name reference chat if one is provided.
- new(chat_name) -> group_id: Create a new chat with given group_id. Ideally, calling MChat(MChat.new()) gives a handle to this new chat.
- add(users)/remove(users)/refill(users): Add/remove users to group. Refill removes all from group not in users, then adds anyone not in the group.
- cast(msg): Format the message such that any instance of a player id surrounded by "[]" square brackets is turned into the name of that player, using this chat if name_reference is None, or the name_reference chat.
- getName(player_id): Get the name of a player according to name reference

The MDM interface includes:
- __init__(name_reference): Create handle to DMs. When placing in names for Player IDs, use the name_reference chat.
- send(msg, user_id): Send a message to the user. Format the message similar to cast would in the name_reference chat

### MServer

This involves a Server that will listen for messages from a platform. For groupme these are bot callbacks. For Discord, we might have to get creative and figure out how to extract and inject async things in Python.

The current MServer interface involves:
-__init__(handle_chat, handle_dm): handle_chat and handle_dm are callbacks with the function signature `(group_id, sender_id, cmd, **kwargs)`. Various commands use the kwargs, which could include the text of a message, the mention of a vote, etc. Essentially, these are either commands associated with a chat or commands associated with a dm. We could potentially simplify it to a single callback.
- run: Runs the server.

Are callback functions the best way to work this system?

### Thoughts

At this point the MServer and MChat/MDM structures are pretty much completely separate, which I think is good.

One question is about the Lobby system. For now, I am assuming the lobby system would be a generic thing that should be implemented for other platforms/game logic. But this isn't necessarily true.

It is good that MGame down is separate from the Lobby, I think. But it also seems like there should be a way to kind of directly connect MServer to MGame. In this way, the lobby would be able to start games, but would not be necessary for routing the server messages to the games.

Idea: Give MServer a mapping of group_id to object. It will call handle_chat for each message to a given group based on that. Similarly, the /focus is for DMs, so there is a list of objects associated with each user_id. (Otherwise, lobby would have to route that.)

Do we need a DMHandler to route DMs correctly? That would intercept certain commands (focus) and route other ones to the correct place... This would just be the controller I think. What if we have multiple controllers?

One issue is that the MServer is completely encapsulated from the game logic. This means for things like menu options using the target command, the processing of these must happen later, when access to the game state is possible. For this reason we also have the following platform dependent structure:

### MGame

MGame is a base class that encompasses MState (which is pure game logic) and game chats and dms. It also does the work of routing commands into MState. Because of this, it must be extended to account for platform dependent ways of sending commands. This includes voting and targeting right now. These must be handled in a class that is aware of game logic, but is also aware of the platform.

## Pub/Sub Architecture

One question is how we could simplify everything with Pub/Sub system. We would have to be careful about having multiple subscribers and duplicating certain things but this might be a good idea.

1. Have a single server that listens for any messages or events, and publishes those. There are also single instances of, for example, a DM handler, that handles a disjoint subset of commands, but also all DMs are routed through it?

2. Have lobbies as another entity that listen for a set of commands sent to that lobby.

3. Have games that can listen for a set of commands sent to a group or sent from dm managers?

Thinking through most types of messages or the lifetime of a game...

Server intercepts DM focus commands, publishes focused DMs and group commands.

Lobby receives group commands to create a game, starts timer, then creates game by starting a new process. _It listens for the end of that game?_

Lobby receives a watch command. Publishes this to the appropriate game?

Game is started, subscribes to messages to certain groups and to DMs focused on that group?

Messages are routed to game, which eventually finishes. It publishes a message about its end, which the lobby will pick up, as well as the DM Handler.