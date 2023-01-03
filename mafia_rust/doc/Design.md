
# Mafiabot Design

Mafiabot is a discord chatbot that facilitates a game of Mafia.

## Gameplay Overview

Mafia is a hidden information game between typically two teams. The "Mafia" and the "Town". The Town are trying to discover who the Mafia are and kill them, while the Mafia know who each other are, and are trying to kill town.

The game commences in Phases, typically alternately Day and Night. During a Day phase, all players may publicly vote for other players. Once a player has received a majority of votes, they are elected and eliminated and the game proceeds to the Night phase. During a Night phase, the Mafia will target somebody to be killed. At the end of Night, the targeted player will be eliminated.

Aside from the Mafia aligned normal MAFIA role and the Town aligned normal TOWN role, there are other special roles, such as COP, DOCTOR, CELEB, GODFATHER, etc. Some roles also have night actions, such as investigating or saving other players. All night actions are selected during the night, then resolved at the end of Night, which happens once all selections have been made. Some roles are also not aligned with either Town or Mafia. These Rogue roles have specific goals, such as getting voted out, protecting another player, or causing another player's death.

## Game Design

- "Block"
    - Happens behind the scenes
    - Night clears block list
    - Night block list carries into day
- "Stun"
    - Notified at start of night, when options are listed
    - Stun list, any Night Targets (or Marks), can only Abstain

- IDIOT!Stun imparts stun at beginning of Night to electors
- GOON always stuns self at beginning of Night
- DOCTOR!Stun...
    - Need a list for stunned doc? Or edit Role?
    - DOCTOR!Stun saves self => DOCTOR!Stunned gets stun at start of night => DOCTOR!Stun

## Architecture

- Server: Interfaces with Discord
- Controller: Sets up and handles Games
- Core: Runs game logic

### Interfaces

- Core
    - Inputs are comm::Command structs
    - Outputs
        - comm::Event structs for game outputs that don't need context to be routed to users
        - Errors returned from handle need context (Who sent wrong command and from where?) To respond
        - access mutex and check Phase status done by exterior thread to update game status

### Module Structure

- Core: Facilitates the game logic.
    - Functionality is all in the `Game` struct
    - Type system for data.
        - Publically accessible data?
        - Vs internal
    - Interface (Commands and Events)

- Core Data:
    - Players* Constructed from outside? So must be public?
        - UID
        - Role
        - ~~Name~~
    - Phase
        - Init: Pre Start?
        - Day:
            - day_no
            - votes `Vec<Pidx, Choice<Pidx>>`
            - blocks
        - Night:
            - night_no
            - strips `Vec<Pidx, Choice<Pidx>>`
            - saves `Vec<Pidx, Choice<Pidx>>`
            - investigations `Vec<Pidx, Choice<Pidx>>`
            - kill `Option<Pidx, Choice<Pidx>>`
        - Dawn?:
            - dawn_no
            - block_map `HashMap<Pidx, Vec<Pidx>>` // blocked -> [strippers]
            - save_map `HashMap<Pidx, Vec<Pidx>>` // saved -> [doctors]
            - investigations `Vec<Pidx, Pidx>`
            - kill `Option<Pidx, Choice<Pidx>>`

#### Contracts

Semantics
- Contracts given at beginning of game
- Type can be `Assassinate` `Protect` `Elect`
- When that is met, (election or elimination)
    - Search contracts for a charge that died/elected
    - Note the outcome on contract?
    - Potentially refocus...

Store in player? Or store in a separate field?
Need separate field, mut retain info after player deaths

How to handle contract wins?

Must announce win/lose for each of them at the end of the game, so do we even need to declare some special win?

#### Rolegen

Want to generate fair roles for a game.

Factors involved in generating roles:
- Number of roles to generate.
- "Roleset", Or roles we are allowed to add
- Ruleset determines the strength of certain roles, or the bias towards/against Town

Process gen
- Max 1 COP/DOC per X in game?
    - Probability distribution...
    - `(N + A) / X where...
        - N is a uniform random number between 0 and number of players
        - A is a const for COP/DOC
        - X is const for COP/DOC
        - For example, if X is 7, A is 3, for a 9 player game...
            - 0 COP: 40%
            - 1 COP: 60%
            - 2 COP: 0%
        - Another example, X: 7, A: 3, a 15 p game
            - 0 COP: 4/16
            - 1 COP: 7/16
            - 2 COP: 3/16
- Also, let other roles generated affect this?
    - How to gen MILLER and GODFATHER relative to COP?
    - Do them before cop. Adjust A based on # of them?

So, for each role we have
- X (Scale)
- A (Offset)
- V (Variance)

To get the number of a role...
- `num_role = (N + V + A) // X`

Now, just figure out how roles affect each other and ordering
- MILLER/GODFATHER
    - Weakens COP, so allow more COPs (A+~4 per?)
- CELEB
    - Just strengthens Town... Lower Town A's and raise Maf A's
- GOON
    - Strengthens Town a lot in small games? Negative offset? Until you get to more than one maf, 
- IDIOT
    - 1/2 maf... High scale
- GF, STR, Maf?

- Assign # of maf.
- Choose types of maf?

- num maf
- MILLER
- GF 
- COP
- CELEB
- Rogue
- STR
- DOC

|Factor| Base X   | V  | a  |
|---|---|---|---|
| Number of Mafia | ~3.5 | 1? | 0 |
| Cop? | 7 | 5 | 4 |




        
