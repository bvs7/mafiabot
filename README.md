# MafiaBot
Bot designed to play the game "Mafia".  Currently testing for GroupMe games, and still developing for discord.

## Teams
### Town
Town is the majority team.  They are trying to find all of the mafia members and kill them.
### Mafia 
Mafia is the secret minority.  They know who the mafia is, and they can kill someone every night.  If they are ever not the minority, they win!
### Rogue 
Rogues aren't on any team, only in the game for themselves.  Their victory conditions vary based on their role.

## Game Structure
The game takes place in cycles of DAY and NIGHT.  During the day, everyone can elect one person to kill.  During the night, the mafia picks one person to kill.

## Roles
### Town
#### TOWN
The TOWN is a normal player in this game, the last line of defense against the mafia scum. They sniff out who the mafia are and convince their fellow town members to kill them during the day!
#### COP
The COP is the one of the most offensive members of the townspeople. During the Night, they send a direct message to MODERATOR with the letter of the person they want to investigate, and upon morning, MODERATOR will tell them whether that person is MAFIA or NOT MAFIA.
#### DOCTOR
The DOCTOR's job is to save the townspeople from the mafia scum. During the Night, they send a direct message to MODERATOR with the letter of the person they want to save. If the mafia targets that person, they will have a near death experience, but survive.
#### CELEB
The CELEB is a celebrity. Everybody knows who they are, but everyone doesn't recognize them right now. CELEB can reveal themselves during Day by sending MODERATOR '/reveal' and then everyone will know they are Town. But they ought to be careful! They'll be quite the target once revealed!
#### MILLER
The MILLER is pretty sus but they are actually on the side of Town... If the cop investigates them, they show up as MAFIA...
#### MILKY
The MILKY gives out some milk to someone every night. Other than that they are a normal townsperson. Don't milk yourself!
### Mafia
#### MAFIA
The MAFIA is part of the mafia chat to talk privately with their co-conspirators. During the Day, they try not to get killed. During the Night, they choose somebody to kill!
#### GODFATHER
The GODFATHER is a leader of the mafia, up to no good! They use the mafia chat to conspire. If a cop investigates them, they'll see the GODFATHER as NOT MAFIA!
#### STRIPPER
The STRIPPER is a member of the Mafia with a special ability. During the night, they can distract one person. This person can't do their job that night (and possibly the following day). A distracted COP learns nothing, a distracted DOCTOR can't save, and a distracted CELEB can't reveal for a full day!
#### GOON
D'oh! The GOON is a member of the Mafia that cannot help target another player in the mafia chat at night. You can notarget but you cannot target another player...

### Rogue
#### IDIOT
The IDIOT's dream is to be such an annoyance that the townsfolk kill them in frustration. They don't care whether the mafia win or lose, as long as everyone votes for them.
#### SURVIVOR
The SURVIVOR's only goal is to survive until the end of the game. Help Mafia? Help Town? It's up to you but make sure you'll live!
#### GUARD
The GUARD is tasked with protecting a charge. You win if that player survives until the end of the game.
#### AGENT
The AGENT is tasked with inviting the death of a charge. Whether by election or by directing murder, you win if your charge dies.
