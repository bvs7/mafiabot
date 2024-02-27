// Controller.

/*
A unique controller will exist for each guild the bot is active in.

The controller is responsible for:
- Maintaining Game Cores
- Routing Core Actions to the appropriate Game Core
- Handling other App Actions
- Handling Game Core Events

Other App Actions:

- Create Lobby (either in new channel or in current channel)
- Close Lobby
- Create new game
  - Create game thread in lobby channel
  - Create Game message with current players
  - Listen for reacts to join game? Or just press button
- Start a game
  - Rolegen
  - Create game channel + threads
- Rules Import/Export/View/Setting

*/
