
Mafia Game:

General:
- Lobby id : Chat ID
  - Game Number (for lobby) : int
- Main Chat id? : Chat ID
- Mafia Chat id : Chat ID
- Day and Phase : Phase
- Players : MPlayer
- Rules : MRules


Special Phase Info:
- Day
  - Votes
  - Stuns (CELEB)
- Night
  - Targets
- Dusk
  - Vengeance


Types:

Roles?
Roles are on a team. Have abilities, trigger in various orders, can have extra data with them?

One conceptualization... Role is Team + Abilities. So you could have a savior (doctor), investigator (cop), revealer (CELEB) on town, or on mafia?

Additionally, you might need data associated with a role, i.e. AGENT or GUARD.

So a player's role could be an instance with some flags and space for data...

MPlayer:
- id : PlayerID
- role : MRole
- (Has a message fn??)

Phase:
- day : int
- phase : Phase
- start_time : datetime

Phase

Day(Phase):
- votes : Dict[PlayerID,Optional[PlayerID]]
- stuns : List[PlayerID]
- cloaks : Dict[PlayerID, RoleClass?]


Modular design?

Interactions:

Explicit game updates sent to player or to main/mafia chats are sent to a update manager, which can be flavored to discord, groupme, etc. (event messages)
Which ones generate messages? Not just respond?
Events:
- Election
- Dawn
- Vote
- Target
- Eliminate
- Refocus
- Strip
- Save
- Kill
- Investigate

Data or info can be polled or requested. (status,)

Day State, votes change etc until notice an election...

Night State, targets change until notice dawn...

Everything else makes small adjustments...

So... for consistency... votes and targets are changed...
and have another process that watches and executes dusk and dawn

Process watches for dusk/dawn every second. After first valid check, waits X amount of time and checks again. If both checks are valid, continue...

Dusk:
- Election Anouncement
- Elimination message
  - DM roles/explanation
  - Announce death and reveal
- Night begins
  - Tell targeters options

Dawn:
- Dawn Announcement
- Strip Messages
- Kill
  - Save Messages
  - Kill Elimination Announce/Reveal
  - DM roles/explanation
- Day begins
  - Tell Main the count


Do we really want the targeting actions distributed? During dawn, we will have a context. We run through each targeting role in priority order, and run a function on the context.

This seems bad. it would probably just be better to have all this code somewhere else, rather than get all weird about it.

When messing with roles, we want to be able to quickly get...
Team
isTargeting
isContract

We want to create role from string? no, we want full object...

Ok ok Role is an enum.
But part of role is the charge if it is a contract role. Other roles might have more info as well.

So have roleinstances.