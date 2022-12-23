# Mafia Game Roles

## Implemented

#### Town Aligned
- TOWN
- COP!(Investigation?)
    - Role | **Team** | Mafia
- DOCTOR!(SaveSelf?)
    - **Always** | Once | Stun | Never
- CELEB
- MILLER
#### Mafia Aligned
- MAFIA
- GODFATHER
- STRIPPER!(StripNotify?)
    - **Useful** | Always
- GOON

#### Rogue (Unaligned)
- IDIOT!(IdiotElect)
    - Win| Cull | **Dusk** | Day | Stun | None

## Planned 
- MASON
- SURVIVOR
- GUARD!(GuardContract?)
    - **Refocus** | Retire
- AGENT!(AgentContract?)
    - Retire  | **Refocus**

## Ideas
- MILKY
- BRUTE
- NEIGHBOR
- MAYOR
- HEIR
- PRESIDENT
- DICTATOR
- LOVER!(LoverContract?)
    - **Refocus** | Retire
- COURTESAN!(CourtesanContract?)
    - **Refocus** | Retire
- VIGILANTE!(VigilanteKill)
    - **Infinite**|Three|Two|One|Half
- KILLER
- WITCH

### Transistions
- VIGILANTE!Three => VIGILANTE!Two => VIGILANTE!One => TOWN
- GUARD(x)!Refocus => AGENT(y)!Refocus => GUARD(z)!Refocus
- GUARD(self)!Refocus -> SURVIVOR
- AGENT(self)!Refocus -> IDIOT!None
- LOVER(x)!Refocus -> AGENT(y)!Refocus
- COURTESAN(x) -> AGENT(y)!Refocus

## Role Explanations
- __TOWN__: A basic townsperson with no special abilities.
- __COP__: At night can target a player to investigate them, learning about what their role is. Investigations can reveal Full Roles, Team Alignment, or jsut whether or not a player is Mafia Aligned. Notably, MILLER and GODFATHER make investigations fallible.
- __DOCTOR__: At night can target a player as a patient to attempt to save them. If the Mafia attacks the patient, they will be Blocked, and the patient will survive.
- __CELEB__: During the day, can reveal themselves as irrefutably CELEB.
- __MILLER__: When investigated by COP, they show up as MAFIA, Team Mafia, or Mafia Aligned.
- __MAFIA__: A basic Mafia Aligned player. At night, the Mafia can conspire. One Mafia Member can target another player to kill them.
- __GODFATHER__: When investigated by COP, they show up as TOWN, Team Town, or Not Mafia Aligned.
- __STRIPPER__: At night can target a player to stun them, blocking their action. For COP, DOCTOR, or a Mafia Killer, this has the same effect as that player targeting nobody that night. For CELEB, they are unable to use their Reveal Action during the following Day Phase.
- __GOON__: A Mafia member who can't kill. When they try to target a kill, it passes as an Abstain choice.
- __IDIOT__: The IDIOT's goal is to be voted out. When they are voted out a few effects that can happen depending on the rules:
    - None: The IDIOT's win will be announced at the end of the game.
    - Win: The IDIOT wins and the game ends.
    - Dusk: The IDIOT selects one person who voted for them. That person is killed before the IDIOT is killed
    - Stun: Everyone who voted for the idiot is unable to act the following Night.
    - Cull: Everyone who voted for the idiot dies.

### Planned Role Explanations
- __MASON__: A group of Masons have a chat where they can discuss the game day or night, and they know the others are Town Aligned.
- __SURVIVOR__: Rogue. They win if they survive to the end of the game. Sort of an alias for GUARD of self.
- __GUARD__ and __AGENT__: Similar roles, each have a "contract" to protect (GUARD) or assassinate (AGENT) another player. A GUARD wins when their charge survives until the end of the game, and an AGENT wins when their charge is killed. This can happen posthumously. There are a few ways to handle these wins though:
    - None: When the charge dies, nothing happens and the GUARD or AGENT is announced as a loser or winner at the end.
    - Refocus: When the charge dies, a living GUARD or AGENT refocuses. A GUARD becomes and AGENT and an AGENT becomes a GUARD. Their new charge is the player that caused the death of the old charge. (In an election, the last person to vote for them or the hammer, if a mafia kill, the Mafia member that performed the kill, if a vengeance kill by an idiot, the charge becomes themself). If the charge would become yourself, a GUARD would not become an AGENT, but an IDIOT, and an AGENT would not become a GUARD, but a SURVIVOR.
    - Retire: When the charge dies, a living GUARD or AGENT effectively dies with them.

### Role Idea Explanations
- __MILKY__: Town. Targets one person (not self) at night. If both MILKY and other person live through the night, it will be announced publicly that that person received milk.
- __BRUTE__: Mafia. Whoever the Brute voted for during the day is Stunned the following night and can't perform their night actions.
- __NEIGHBOR__: Town. Targets one person in the night. That person is informed that the NEIGHBOR is NEIGHBOR.
- __MAYOR__: Town. Targets one person in the night. During the next Day Phase, that person takes one less vote to kill.
- __HEIR__: Town. If an HEIR is alive, the usual Mafia win condition is suspended. 
- __PRESIDENT__: Town If a PRESIDENT dies, Mafia wins.
- __DICTATOR__: Mafia. If a DICTATOR dies, Town wins.
- __LOVER__ and __COURTESAN__: Rogue. Always come in pairs (can be LOVER-LOVER, LOVER-COURTESAN, or COURTESAN-COURTESAN). COURTESANs are in the Mafia channel. These are basically two GUARDs that have each other as charges.
- __VIGILANTE__: Town. Can target at night to kill somebody. Can come in a few flavors:
    - Normal: No restrictions.
    - X Shot: After killing X times, becomes Town.
    - Half: After killing, you are stunned the next night.
    - Guess: Also supply a guess as to the target's Role (not MAFIA). If correct, that target dies. If incorrect, the VIGILANTE dies.
- __KILLER__: Rogue. Can target at night to kill somebody. When a KILLER is alive, the normal Town and Mafia win conditions are suspended
- __WITCH__: Mafia. Can target someone at night, and guess their role (cannot guess TOWN). If they are correct, that person dies. If they are incorrect, the WITCH is publicly declared at the start of the next day.
