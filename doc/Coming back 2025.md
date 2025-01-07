
## Splitting app up
Idea: Split the app into specific parts...

The core runs on an http server that can take in commands and return status
Additionally, have an event queue that lets things subscribe and broadcasts events?

Core:
- Starting a game. 
	- Spawn a server at some local port number...
	- Get rules
	- Rolegen
	- Assign users roles to create players
	- Start first phase

Why would we want to isolate the core like this?
- Standalone core can continue running if client doesn't persist.
- A uniform API for the core decouples the "front end" (discord) and the back end.
- Multiple things can interact with a standalone core
- Testing is simpler and more representative


### HTTP server (And API)
Create an API with following endpoints:
- GET /games/{gameid} => Get game status (including full event list?)
- GET /games/{gameid}/events => Get all events in game
	- after? query string param is an event id, only events after this one are returned
	- limit? is a number, default 0. Return max this many events. 0 means no limit.
- POST /games/{gameid} => Send an action to the game

What is included in a game:
Game
- Game Id
- State
	- Phase
		- Day: votes/blokcs
		- Night: targets/scheme
		- Eclipse: avenger/hammer/options
		- Init/End: winning_team
	- Players (user -> role)
	- Role history (user -> vec\<role\>) (used to determine Rogue wins, and who played and died)
- Rules
- Actions?
- Events

## Actions:
- Vote
- Reveal
- Target
- Scheme
- Avenge?
- Elect? Based on rules?

## Features
Think about how to add or remove players mid-game. One method is replacing, but others should be available. Maybe have some kind of check-in feature, and any who fail to check in are kicked? Otherwise, when players are added... maybe they could have some temporary role, then be assigned the following phase? In a known game, this is difficult.... hmmm... 

Maybe there are weaker roles, that can be added without upsetting the game balance much? Roles that can't vote? Or who only get half a vote? Hmmm...

## Calculating probabilities of wins?

Simulate player actions.
- Simulate player knowledge: Each other player has probabilities of each role they could be.
- Note: Mafia has shared knowledge!
- Day things are... possibly claim a role, start a vote, share info?
- Night things are... target.

Player responses
- To a claim, possibly counterclaim
	- If they have claimed your role... definitely counterclaim
	- If you are mafia, possibly counterclaim
- To a vote, possibly bandwagon

How do different things affect knowledge?
- Claim a role
	- Mafia updates with high certainty from a town claim
	- Town updates with high certainty, unless counter-claim