
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

Rolegen is done in two phases:

- First, the number of rogue then number of mafia players out of the number playing. The rest will be town.
    - This is done using an equation: `n_maf = (n_players + R) / 3.0` where R is a normal random variable with mu 0.0 and sigma 1.5
    - Number of rogue roles is determined by `n_rogue = (ln(n_players) * R) / 8.0` with R being normal(3.0, 5.0)
- Second, the rogue roles are determined.
    - Each possible Rogue role has a +/- town modifier associated with it. GUARDs with a Town charge and AGENTs with a Mafia charge are positive (good for town) while IDIOTs, GUARDs with Mafia charges, and AGENTs with Town charges are negative (bad for town). 
    - The possible values for the Rogue roles are generated 10 times and compared to a quick hieuristic from the Town vs Maf matchup. The most fair generated set wins.
- Third, the team roles are determined.
    - Using more normal distributions with specific equations to tune the odds, the number of each possible special roles are generated.
    - **Idea** Similarly to the Rogue role generation, generate X number of sets of team roles. Use a hieuristic to determine which set seems best.
    - **Idea** Generate a "spiciness" level. Generate sets of Town teams and Mafia teams. Find the one that best matches spiciness levels.
        - Spiciness could be a percentage? 0% means all TOWN/MAFIA. 100% means all special roles?

Main ideas for rolegen:
- Nothing (within reason) should be specifically "impossible". But some things will be very unlikely.
    - Use normal distributions so that extreme outliers are possible
    - Numbers of Teams determined by normal distribution
    - Using Spiciness, determine a number of special roles?
        - Randomly generate spicy roles 2*n times. Use hieuristic to determine the best. Backfill with basic TOWN and MAFIA roles?
        - Use Team numbers in hieuristic calculation, too.

Spice Method
- Determine # of Spicy roles
- Determine # of Rogue roles (spicy)
    - Generate sets of rogue roles
    - Order them
- Generate sets of spicy roles
    - Randomly assigned from probabilities.
    - Order them?
- Generate sets of maf-town split
    - Order them?
- Now we have 3 ordered sets of collections
    - Take the median of each
    - Loop of the following X number of times?
        - Find score,
        - Adjust a random of the three up or down

Ideas for rolegen
- Full Generation: Gen a bunch of full games fully randomly, and choose the best
- Partial Gen: Calculate some parameters, then generate sets and choose the best
    - Number of Mafia
    - Number of Rogue
- Bit by bit: Calculate full game one thing at a time

- Spice
- Fairness
- Number of Players

- Calculate number of Mafia
    - Higher spice value leads to more mafia
- Calculate number of Rogue
    - Higher spice value leads to more as well?

How spicy are the following roles?
- COP. Full spice
- DOCTOR. Full spice
- CELEB. Mostly spice
- MILLER. With COP, a little spice
- GODFATHER. With COP, a little spice
- STRIPPER. With any COP, DOCTOR, CELEB, Full spice

Calculate Spice value
- Grab one role at a time until spice level is met.
- Fill in with remaining mafia and rogue to optimize score?
- First add rogue, if it is involved, as that might be +/- limited based on roles...
- Then fill in remaining TOWN, MAFIA
- Potentially replace MAFIA with GOON? If town needs some help?

Can't determine spice of GOON until later...
    

        
