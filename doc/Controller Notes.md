
Discord events are passed into the Controller, if they take place in the Controller's Guild
Also, the server should filter things by the mafia bot application.

- `interaction_create`
	- Button presses
		- Reveal button in a reveal thread
		- join game button in a game initializer
		- start game button in a game initializer
		- watch game button
	- Select Menu (string?)
		- target in a target thread
		- scheme in the mafia thread?
	- Slash Commands
		- Lobby create/open/close
		- game create
		- vote
And is that it? Is that all that is needed?

Should we route things specifically?

Join Game and Start Game and watch game can go in the Game Initializer.

Game commands could go to the game? Or to a wrapper around it...
	Game Handler. Takes action inputs and handles event handler output. Needs an http reference?

Lobby commands act within the controller?

game commands act within the lobby?


Use collectors to filter out relevant commands!

Filters:
- Interaction

- Application Id -> Mafia Bot

- Command
	- mafia
		- lobby
			- open -> Channel ID
			- close -> Channel ID (Lobby)
			- create -> ?
		- game
			- create -> Channel ID (Lobby)
- Component
	- Button
		- Join Game -> Game Initializer, User ID
		- Start Game -> Game Initializer,
		- Watch Game -> Game Initializer, User ID
		- Reveal -> Game, User ID
	- Select Menu
		- Target -> Game, User ID, Choice
		- Scheme -> Game, User ID, Choice

Have a universal error response method for interactions

Message::await_component_interaction
Used to create a thread to wait for join game and start game button presses??