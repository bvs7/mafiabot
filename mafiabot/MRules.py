
from typing import Set, Union
import copy

from .MPlayer import TOWN_ROLES, MAFIA_ROLES, ROGUE_ROLES, ALL_ROLES

ROLE_NOTES ={
  "TOWNS" : set(TOWN_ROLES),
  "MAFIAS" : set(MAFIA_ROLES),
  "ROGUES" : set(ROGUE_ROLES),
  "SIMPLE" : {"TOWN", "COP", "DOCTOR", "MAFIA"},
  "MEDIUM" : {"TOWN", "COP", "DOCTOR", "CELEB", 
              "MAFIA", "GODFATHER", "STRIPPER", 
              "IDIOT"},
  "DONE "  : {"TOWN", "COP", "DOCTOR", "CELEB", "MILKY",
              "MAFIA", "GODFATHER", "STRIPPER", "GOON",
              "IDIOT", "SURVIVOR", "GUARD", "AGENT"},
  "ALL" : set(ALL_ROLES),
}

class MRoles:

  def __init__(self, noteSet : Union[str,Set[str]] = ALL_ROLES):
    self.roles = self.noteRoles(noteSet)

  def __iter__(self): # For ordering roles
    return iter([r for r in ALL_ROLES if r in self.roles])

  def noteRoles(self, noteSet : Union[str,Set[str]]) -> Set[str]:
    if type(noteSet) == str:
      noteSet = {noteSet}
    roleSet = set()
    removeSet = set()
    nrset = roleSet
    for note in noteSet:
      if note[0] == "-":
        nrset = removeSet
        note = note[1:]
      else:
        nrset = roleSet

      if note in ROLE_NOTES:
        nrset.update(ROLE_NOTES[note])
      elif note in ALL_ROLES:
        nrset.add(note)

    roleSet.difference_update(removeSet)
    return roleSet

  def addRole(self, noteSet : Union[str,Set[str]]):
    self.roles.update(self.noteRoles(noteSet))

  def removeRole(self, noteSet : Union[str,Set[str]]):
    self.roles.difference_update(self.noteRoles(noteSet))


RULE_LIST = [
  "known_roles",
  "reveal_on_death",
  "start_night",
  "know_if_saved",
  "know_if_saved_doc",
  "know_if_saved_self",
  "cop_strength",
  "idiot_vengence",
  "charge_refocus_guard",
  "charge_refocus_agent",
  "know_if_stripped",
  "no_milk_self",
]

RULE_BOOK ={
  "known_roles" : {
    "ROLE" : "Role list of living players is common knowledge",
    "TEAM" : "Number of players with each alignment (Town, Mafia, Rogue) is common knowledge",
    "MAFIA" : "Number of Mafia aligned and Not Mafia aligned players is known",
    "OFF": "No role info known",
  },
  "reveal_on_death" :{
    "ROLE" : "When a character dies, their role is revealed",
    "TEAM" : "When a character dies, their alignment (Town, Mafia, Rogue) is revealed",
    "MAFIA" : "When a character dies, they are revealed as Mafia aligned or Not Mafia aligned",
    "OFF" : "When a character dies, nothing is revealed",
  },
  "know_if_saved" : {
    "ON" : "The success of a doctor's save is publicly revealed",
    "OFF" : "The success of a doctor's save is NOT publicly revealed",
  },
  "know_if_saved_doc" : {
    "ON" : "The success of a doctor's save is revealed to the doctor",
    "OFF" : "The success of a doctor's save is NOT revealed to the doctor",
  },
  "know_if_saved_self" : {
    "ON" : "The success of a doctor's save is revealed to the target",
    "OFF" : "The success of a doctor's save is NOT revealed to the target",
  },
  "start_night" :{
    "ON" : "The game starts in the Night phase",
    "EVEN" : "If there are an even number of players, the game starts in the Night phase",
    "ODD" : "If there are an odd number of players, the game starts in the Night phase",
    "OFF" : "The game starts in the Day phase",
  },
  "charge_refocus_guard" :{
    "ON" : "When a GUARD's charge is killed, the GUARD refocuses, becoming an AGENT charged to kill the one responsible for the death",
    "DIE" : "When a GUARD's charge is killed, the GUARD immediately dies",
    "OFF" : "When a GUARD's charge is killed, nothing else happens",
  },
  "charge_refocus_agent" :{
    "ON" : "When an AGENT's charge is killed, the AGENT refocuses, becoming a GUARD charged with protecting the one responsible for the death",
    "WIN" : "When an AGENT's charge is killed, the AGENT immediately wins, effectively dying",
    "OFF" : "When an AGENT's charge is killed, nothing else happens",
  },
  "idiot_vengence" :{
    "ON" : "When an IDIOT is elected, they must select one character that voted for them to also die",
    "WIN" : "When an IDIOT is elected, they immediately win and everyone else loses",
    "DAY" : "When an IDIOT is elected, the Day phase continues, allowing for a second election",
    "STUN" : "When an IDIOT is elected, all characters that voted for them are stripped",
    "OFF" : "When an IDIOT is elected, nothing else happens",
  },
  "know_if_stripped" :{
    "ON" : "Any character targeted by a STRIPPER, learns they were stripped the next morning",
    "TARGET" : "Any targeting role (COP, DOCTOR, MILKY, CELEB, STRIPPER) is informed if they were stripped the next morning",
    "USEFUL" : "On a useful strip (target is prevented from doing something) the target learns they were stripped the next morning",
    "OFF" : "A character targeted by a STRIPPER never learns they were stripped, even if the side-effects are obvious (COP learns nothing, DOC save failed, CELEB's reveal doesn't work, etc.)",
  },
  "no_milk_self" : {
    "ON" : "A MILKY cannot target themself at night",
    "OFF" : "A MILKY can target themself at night",
  },
  "cop_strength" : {
    "ROLE" : "A COP learns the role of the character they investigate (GODFATHER -> TOWN, MILLER -> MAFIA)",
    "TEAM" : "A COP learns the alignment of the character they investigate (Town, Mafia, Rogue) (GODFATHER -> Town, MILLER -> Mafia)",
    "MAFIA" :"A COP learns whether their target is Mafia aligned or Not Mafia aligned"
  },
  "roleSet": "A set of the roles to be used in rolegen",
}

class MRules:

  default_rules = {
    "known_roles":"TEAM",
    "reveal_on_death":"TEAM",
    "know_if_saved":"OFF",
    "know_if_saved_doc":"ON",
    "know_if_saved_self":"ON",
    "start_night":"EVEN",
    "charge_refocus_guard":"ON",
    "charge_refocus_agent":"ON",
    "idiot_vengence":"ON",
    "know_if_stripped":"USEFUL",
    "no_milk_self":"ON",
    "cop_strength":"MAFIA",
  }

  def __init__(self, noteSet : Union[str,Set[str]] = 'MEDIUM'):
    self.rules = self.default_rules.copy()
    self.roles = MRoles(noteSet)


  def __getitem__(self, item):
    if item in self.rules:
      return self.rules[item]
    raise AttributeError(item)

  def __setitem__(self, name, value):
    if name in RULE_BOOK:
      if value in RULE_BOOK[name]:
        self.rules[name] = value
      else:
        raise ValueError("No {} setting for rule {}:({})".format(value, name, '|'.join(RULE_BOOK[name].keys())))
    else:
      raise KeyError("No rule {}".format(name))

  def __str__(self):
    rule_things = []
    for rule in RULE_LIST:
      sett = self.rules[rule]
      expl = RULE_BOOK[rule][sett]
      rule_things.append( (rule, sett, expl) )

    col_len = max([len(t[0]) + len(t[1]) for t in rule_things])

    msg = ""
    for (rule, sett, expl) in rule_things:
      msg += rule
      msg += "-{:->{w}}|".format(sett, w=col_len-(len(rule)))
      msg += expl
      msg += '\n'

    msg += "\nroleSet:  " + ", ".join(self.roles)
    return msg

  @staticmethod
  def explRule(rule):
    msgs = []
    sett_col_len = max([len(s)+1 for s in RULE_BOOK[rule]])
    for sett in RULE_BOOK[rule]:
      msgs.append("{:{w}}|".format(sett,w=sett_col_len) + RULE_BOOK[rule][sett])
    return '\n'.join(msgs)
