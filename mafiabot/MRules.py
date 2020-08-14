from typing import Set, Union
import copy

from .MInfo import *

RULE_LIST = [
  "known_roles",
  "reveal_on_death",
  "start_night",
  "know_if_saved",
  "know_if_saved_doc",
  "know_if_saved_self",
  "idiot_vengeance",
  "charge_refocus_guard",
  "charge_refocus_agent",
  "know_if_stripped",
  "no_milk_self",
  "cop_strength",
  "unique_night_act"
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
    "SAVED" : "Upon a successful save, the saved character is revealed",
    "SECRET" : "Upon a successful save, the fact that a save was successful is revealed, but no other information",
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
  "idiot_vengeance" :{
    "KILL" : "When an IDIOT is elected, they must select one character that voted for them to also die",
    "WIN" : "When an IDIOT is elected, they immediately win and everyone else loses",
    "DAY" : "When an IDIOT is elected, the Day phase continues, allowing for a second election",
    "STUN" : "When an IDIOT is elected, all characters that voted for them cannot act during the night.",
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
  "unique_night_act" :{
    "ON" : "A player can only act once per night (STRIPPER cannot both mafia target AND strip target)",
    "OFF" : "A player can perform all of their night actions each night",
  }
}

class MRules:

  default_rules = {
    "known_roles":"TEAM",
    "reveal_on_death":"TEAM",
    "know_if_saved":"OFF",
    "know_if_saved_doc":"ON",
    "know_if_saved_self":"OFF",
    "start_night":"EVEN",
    "charge_refocus_guard":"ON",
    "charge_refocus_agent":"ON",
    "idiot_vengeance":"KILL",
    "know_if_stripped":"USEFUL",
    "no_milk_self":"ON",
    "cop_strength":"MAFIA",
    "unique_night_act":"OFF"
  }

  def __init__(self):
    self.rules = self.default_rules.copy()


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
      msg += "-" + sett
      msg += ":" + expl
      msg += '\n'

    return msg

  def describe(self, has_expl=True):
    rule_things = []
    for rule in RULE_LIST:
      sett = self.rules[rule]
      expl = RULE_BOOK[rule][sett]
      rule_things.append( (rule, sett, expl) )

    msg = ""
    for (rule, sett, expl) in rule_things:
      msg += rule
      msg += ":" + sett
      if has_expl:
        msg += "|" + expl
      msg += '\n'
    return msg

  @staticmethod
  def explRule(rule, curr_sett=None):
    """ return a string explaining the possible settings for rule """
    msgs = []
    for sett in RULE_BOOK[rule]:
      if curr_sett == sett:
        msgs.append("[{}]|".format(sett) + RULE_BOOK[rule][sett])
      else:
        msgs.append("{}|".format(sett) + RULE_BOOK[rule][sett])
    return '\n'.join(msgs)
