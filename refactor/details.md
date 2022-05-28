## Roles
- Team Town
  - TOWN
  - COP
  - DOCTOR
  - CELEB
  - MILKY
  - MILLER
- Team Mafia
  - MAFIA
  - GODFATHER
  - STRIPPER
  - GOON
- Team Rogue
  - IDIOT
  - SURVIVOR
  - GUARD
  - AGENT

### Role Ideas

- MASON: Town. Masons know each other as Town from the beginning of the game
- BRUTE: Mafia. Whoever they end the day voting for is stunned that night and following day.
- NEIGHBOR: Town. Targets at night to tell someone their role.


## Rules:

- start_night: What phase do players start in?
  - ALWAYS: Always start in night phase
  - NEVER: Never start in night phase
  - [EVEN]: Start Night when there are an even number of players
  - ODD: Start Night when there are an odd number of players

- save_info_public: What info is shared to the public about a doctor save
  - [NONE]: A successful save is indistinguishable from a mafia no target
  - ANON: The public learns there was a successful doctor save
  - PAT: The patient is revealed on a successful save
  - ALL: The saving doctor and patient are revealed

- save_info_private: What info is shared to private parties about a doctor save
  - NONE: No info shared on a successful save (to private parties)
  - PAT: Patient learns they were saved on a successful save
  - DOC: Doctor learns if their save was successful
  - MAF: Mafia is told if their kill target was saved
  - (COMBO), can combine above

- death_info: What info is revealed on death
  - NONE: Absolutely no info about who died
  - [MAF]: Reveals if the dead is Mafia or Not Mafia
  - TEAM: Reveals the Team (Town, Mafia, Rogue) of the deceased
  - ROLE: Reveals the Role of the deceased

- start_info: What info is revealed on start
  - NONE: Absolutely no info at the start of the game
  - MAF: Reveals number of Mafia and Not Mafia at start
  - [TEAM]: Reveals number on each team at start
  - ROLE: Reveals number of each role at start

- investigate_info: What info is revealed on investigation
  - [MAF]: Investigation yields Mafia or Not Mafia
  - TEAM: Yields Team
  - ROLE: Yields Role

- idiot_event: What happens when idiot is elected
  - NONE: Game continues as normal
  - [DUSK]: Idiot picks a voter to revenge kill
  - STUN: All those who voted for idiot are stunned that night (can only target NOTARGET)
  - CULL: All those who voted for idiot are killed
  - WIN: Game ends and Idiot wins


## Events

Events?

It would be nice to have discrete events that could be sent to the chat controller...

Events:
- Vote
- Elect
- Target
- Mafia_Target
- Block
- 
