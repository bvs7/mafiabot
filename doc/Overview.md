### Description
Mafia Bot is a rust program lets users play a game of Mafia (the party game) via a group chat. Currently, the goal is to use Discord.

## Parts
The total structure of the project is made of a few parts:
- Core: The core mafia gameplay. Platform agnostic
	- The Core receives "Core Actions" as inputs and queues up "Core Events" which are sent to the audience.
- Controller: The service which maintains app state, including status of lobby chats, current games, timers, etc. Interactions from Discord will be routed here which can become a controller command or can be routed into a Core Action for a Core.

## Commands and Actions
There are a few different categories of ways to interact with the bot.
- App Interactions: slash commands and component interactions that allow discord users to interact with the bot
	- /mafia help: help message
	- /mafia new-game: Create a game initializer thread
	- Join Button: Join a game that will start soon
	- Leave Button: Leave a game that will start soon
	- Start Button: Start the game!
	- Watch Button
	- /vote \[mention user\]: vote for someone during the day
	- Target Menu
	- Reveal Button


## Core Functions

Core Actions
- vote
- target
- reveal

Actions that might be spawned by another action

- election
- dawn
- eliminate

## Dawn Ordering

1. STRIPPERs apply blocks
2. DOCTORs apply saves
3. Mafia Scheme is resolved
  - If Doctor(s) saved
    - If a doctor was blocked notify
  - If kill was successful
    - eliminate
  - If save was successful
    - notify
4. COPs apply investigations
  - If COP is still alive and target is still alive
  - If COP was blocked
    - notify
  - if not blocked
    - investigate

So could dawn just be a queue of things?

1. Queue STRIPs
2. Queue SAVEs
3. Queue Scheme
4. Queue INVESTIGATIONs

What info needs to be maintained over this?
- block list (who blocked whom)
- save list (who saved whom)

These could just be filtered out of targets?

resolve scheme:
- Check for DOCTORs that targeted the mark
- For any DOCTOR that targeted the mark, if that doctor was not blocked, the kill is prevented.
	- *Notify successful DOCTOR*
- If the kill was not prevented
	- Eliminate the player
	