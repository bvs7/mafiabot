from ..util import VEnum, auto

class MTeam(VEnum):
  Town = auto()
  Mafia = auto()
  Rogue = auto()

  def __str__(self):
    return self.name

  def to_json(self):
    return self.name
  
  @staticmethod
  def from_json(d):
    return getattr(MTeam, d['__MTeam__'])

TOWN_ROLES = {'TOWN','COP','DOCTOR','CELEB','MILLER','MILKY','MASON',}
MAFIA_ROLES = {'MAFIA','GODFATHER','STRIPPER','GOON',}
ROGUE_ROLES = {'IDIOT','SURVIVOR','GUARD','AGENT',}
TARGETING_ROLES = {'COP','DOCTOR','MILKY','STRIPPER',}
CONTRACT_ROLES = {'IDIOT','SURVIVOR','GUARD','AGENT',}
ROLE_EXPLAIN= {
  'TOWN' : ("The TOWN is a normal player in this game, the last line of "
    "defense against the mafia scum. They sniff out who the mafia are and "
    "convince their fellow town members to kill them during the day!"),
  'COP'  : ("The COP is the one of the most offensive members of the "
    "townspeople. During the Night, they send a direct message to MODERATOR "
    "with the letter of the person they want to investigate, and upon "
    "morning, MODERATOR will tell them whether that person is MAFIA or NOT "
    "MAFIA."),
  'DOCTOR': ("The DOCTOR's job is to save the townspeople from the mafia "
    "scum. During the Night, they send a direct message to MODERATOR with "
    "the letter of the person they want to save. If the mafia targets that "
    "person, they will have a near death experience, but survive."),
  'CELEB' : ("The CELEB is a celebrity. Everybody knows who they are, but "
    "everyone doesn't recognize them right now. CELEB can reveal themselves "
    "during Day by sending MODERATOR '/reveal' and then everyone will know "
    "they are Town. But they ought to be careful! They'll be quite the "
    "target once revealed!"),
  'MILLER' : ("The MILLER is pretty sus but they are actually on the side of "
    "Town... But if the cop investigates them, they show up as MAFIA!"),
  'MILKY'  : ("The MILKY gives out some milk to someone every night. Other "
    "than that they are a normal townsperson. Don't milk yourself!"),
  'MASON'  : ("The MASONs have friends. All MASONs know each other and are "
    "Town Aligned! You are explicitly allowed to private message each other."),
  'MAFIA' : ("The MAFIA is part of the mafia chat to talk privately with "
    "their co-conspirators. During the Day, they try not to get killed. "
    "During the Night, they choose somebody to kill!"),
  'GODFATHER' : ( "The GODFATHER is a leader of the mafia, up to no good! "
    "They use the mafia chat to conspire. If a cop investigates them, "
    "they'll see the GODFATHER as NOT MAFIA!"),
  'STRIPPER' : ("The STRIPPER is a member of the Mafia with a special "
    "ability. During the night, they can distract one person. This person "
    "can't do their job that night (and possibly the following day). A "
    "distracted COP learns nothing, a distracted DOCTOR can't save, and a "
    "distracted CELEB can't reveal for a full day!"),
  'GOON' : ("D'oh! The GOON is a member of the Mafia that cannot help target "
    "another player in the mafia chat at night. You can notarget but you "
    "cannot target another player..."),
  'IDIOT' : ("The IDIOT's dream is to be such an annoyance that the townsfolk "
    "kill them in frustration. They don't care whether the mafia win or lose, "
    "as long as everyone votes for them."),
  'SURVIVOR' : ("The SURVIVOR's only goal is to survive until the end of the "
    "game. Help Mafia? Help Town? It's up to you but make sure you'll live!"),
  'GUARD' : ("The GUARD is tasked with protecting a charge. You win if that "
    "player survives until the end of the game."),
  'AGENT' : ("The AGENT is tasked with inviting the death of a charge. "
    "Whether by election or by directing murder, you win if your charge dies."),
}

class MRole(VEnum):
  TOWN = auto()
  COP = auto()
  DOCTOR = auto()
  CELEB = auto()
  MILLER = auto()
  MILKY = auto()
  MASON = auto()
  MAFIA = auto()
  GODFATHER = auto()
  STRIPPER = auto()
  GOON = auto()
  IDIOT = auto()
  SURVIVOR = auto()
  GUARD = auto()
  AGENT = auto()

  def is_town(self):
    return self in TOWN_ROLES
  
  def is_mafia(self):
    return self in MAFIA_ROLES

  def is_rogue(self):
    return self in ROGUE_ROLES

  def is_targeting(self):
    return self in TARGETING_ROLES

  def is_contract(self):
    return self in CONTRACT_ROLES
  
  def expl(self):
    return ROLE_EXPLAIN[self]

  def investigate(self, cop_strength):
    r = self
    if r == MRole.MILLER:
      r = MRole.MAFIA
    if r == MRole.GODFATHER:
      r = MRole.TOWN
    if cop_strength == "ROLE":
      return r.name
    if cop_strength == "TEAM":
      return r.team().name + " Aligned"
    if cop_strength == "MAFIA":
      if r.team() == MTeam.Mafia:
        return "Mafia Aligned"
      else:
        return "Not Mafia Aligned"

  def team(self):
    if self.is_town():
      return MTeam.Town
    elif self.is_mafia():
      return MTeam.Mafia
    elif self.is_rogue():
      return MTeam.Rogue

  def __str__(self):
    return self.name

  def to_json(self):
    return self.name
  
  @staticmethod
  def from_json(d):
    return getattr(MRole, d['__MRole__'])