
## Ideas

Current setup.

To start the game, we pass ownership of the game struct to itself in the start fn. THis seems pretty necessary?

One question is, how will we update status?

Status mostly consists of Phase:
- Day/Night
- Day number
- votes

Not strictly necessary to have game run by a thread.....

Could pass commands in potentially. Handle a day, (events going out are still necessary). But status can be queried more easily.

No game thread. Just a `fn handle(&mut self, cmd: Command<U,S>) -> Phase<U>`. Discord handler can get back status after every handle, and write that (potentially after pulling off all events from queue)


### Paradigm

- Server receives discord bot http info.
- Passes info to game controller.
- Parses info into a controller access, or into a *Game Command*
- Calls Game Command on Game. Returns status. 
- Pulls from Event Channel and pushes out the appropriate responses
- Updates game status, pushes that.

No Game thread!

## TODO!
Make events push Players, not just Pidx.


## Ideas

Current setup.

To start the game, we pass ownership of the game struct to itself in the start fn. THis seems pretty necessary?

One question is, how will we update status?

Status mostly consists of Phase:
- Day/Night
- Day number
- votes

Not strictly necessary to have game run by a thread.....

Could pass commands in potentially. Handle a day, (events going out are still necessary). But status can be queried more easily.

No game thread. Just a `fn handle(&mut self, cmd: Command<U,S>) -> Phase<U>`. Discord handler can get back status after every handle, and write that (potentially after pulling off all events from queue)


### Paradigm

- Server receives discord bot http info.
- Passes info to game controller.
- Parses info into a controller access, or into a *Game Command*
- Calls Game Command on Game. Returns status. 
- Pulls from Event Channel and pushes out the appropriate responses
- Updates game status, pushes that.

No Game thread!

## TODO!
- Make events push Players, not just Pidx.
- Mutex for game? If commands aren't coming through a channel, we need to worry about concurrency from a server.
- Make InvalidCommand Event an error returned from handle()

## Game Core

The Mafia Game core runs a game of mafia. It is passed an input Command, and returns a current status as well as pushing Events to a channel. These events contain all of the information needed to keep players up to date, log player activities in a database, etc.

### Commands

Current commands are:
- `Vote (voter: UID, ballot: Option<Ballot<UID>>`. Expresses a vote or the retraction of a vote. Votes can be cast during the Day Phase. The day can end when either a hard majority of players vote for another player, eliminating them, or when a soft majority vote to Abstain, and nobody dies.
    - **Idea**. Instead of instantly electing when a vote threshold is reached, start a very short timer (around 10 seconds). If any vote is retracted or changed, this timer stops. Once time is up the election continues. Similarly, for the end of night. have a random timer between 10s to 1 min after last night action is submitted to prevent metagaming.
- `Reveal (celeb: UID)`. A Celeb can reveal themselves. This will send a public message and make it public record that that player is Town Aligned.
- `Action (actor: Actor<UID>, target: Target<UID>)`. Expresses a night action. These actions are either done by a single mafia player (GOON can't kill at night), or by each role with a night action. Actions are not public. Once all night actions (each role with a night action + mafia) have submitted their night actions, the night ends.

Night actions can be one of the following and are effected in the following order:
- Strip. A Stripper picks a target to distract. This blocks them from performing their night action, making it as if they chose "No Target". That player is informed that they were distracted only if it has a noticable effect. When a CELEB is targeted, they cannot use their Reveal action during the next day.
- Save. A Doctor picks a player to save. If that player was also the target of the mafia's kill attempt, they do not die. The doctor alone will be informed of a successful attempt.
- Investigate. A Cop picks a player to investigate. They learn what team (Town, Mafia, or Rogue) the suspect belongs to.. Notably, the MILLER role, which is Town Aligned, shows up as Mafia when investigated, and the GODFATHER role, which is Mafia Aligned, shows up as Town Aligned.
- Kill. A Mafia Team member (except GOON) can choose a target for the Mafia to kill at night. Unless blocked by stripper or doctor, that target dies at dawn.

### Winning

The game ends when either:
- There are as many or more Mafia Aligned Players alive as there are Not Mafia Aligned Players. (Mafia Wins)
- There are no Mafia Aligned Players alive (Town Wins)
- **Idea**: The game can possibly end in a draw in one of two scenarios:
    - After the night of the Nth day where N is the number of players that started the game, if nobody dies for a full Day + Night, the game ends in a draw.
    - If nobody dies for 3 consecutive days, the game ends in a draw.

### Rogue Goals
Aside from the Town and Mafia Team win conditions, there are also individual win conditions for the Rogue roles, which are not aligned with any team. When these win conditions are met, nothing is announced and the game continues secretly.

- IDIOT: The IDIOT wins if they get voted out by the other players. There are a few ways to react to this situation. These ideas are meant to prevent a quid pro quo scenario where the IDIOT can encourage others to vote for them by sharing their role
    - Block. Everyone who voted for the idiot is blocked from taking night actions the following night.
    - Vengeance. The IDIOT selects one of the players that voted for them. That player also dies.
    - Skip. After voting out the IDIOT, the next Night phase is skipped.
    - Cull. All players that voted for the IDIOT are killed.
- SURVIVOR: The SURVIVOR's goal is to survive until the end of the game.
- GUARD: The GUARD is assigned a charge, which is another player they are responsible for. They win if that player survives until the end of the game. Even if the GUARD dies, if their charge survives, they win.
    - Refocus. If your charge dies, you become an AGENT. The person responsible for the death (last vote if elected, mafia targeter if targeted) becomes your charge and you must kill them. If you are responsible, you become an IDIOT (not and AGENT targeting yourself)
    - Die. If your charge dies, you die immediately.
- AGENT: You are assigned a charge, which is another player you are trying to kill. You win if that player dies. Even if you die, you win if your charge dies during the game.
    - Refocus. If your charge dies, you become a GUARD. You are charged with protecting whoever caused the death. If you directly caused the death yourself, you become a SURVIVOR

### Events
When an action is handled by the game, it generates one or more Events. These events contain info which might be publicly or privately revealed, and are what facilitate the game.

- Start. Announces the start of the game, says who is playing and reveals some amount of info about the rolegen
- Day. Announces the start of a Day Phase. Mentions how many players are alive, and how many votes are needed to kill
- Reveal. Publically reveals a CELEB
- Vote. Announces a vote that was cast, updates everyone on the vote tally and relays how many more votes are needed to kill
- Retract vote. "{} retracts vote for {}"
- Election. Announces that the vote threshold has been met and a player or nokill has been elected.
- Eliminate. Announces that a player has died, and reveals information about their role.
- Night. Announces the start of the night phase, and informs those with night roles that they will need to perform them
- Dawn. Announces that night is ending
- Strip. Informs a player that their action was blocked
- Save. Informs a doctor that their save was successful
- Investigate. A Cop gets some role information about the player they investigated.
- Kill. Announces that the mafia has killed someone
- NoKill. Announces that nobody was killed in the night.
- End. Announces the end of the game and who won
...
- Dusk. Announces that the IDIOT was killed, and that they will take revenge
- Vengeance. Announces who the IDIOT picked to take with them
