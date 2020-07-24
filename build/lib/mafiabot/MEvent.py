from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET

# TODO: COP/GODFATHER/MILLER
# TODO: safety checking inputs


class MPhase(Enum):
  INIT = auto()
  DAWN = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()

class EndGameException(Exception):
  pass


default_resp_lib = {
  "VOTE_RETRACT": "[{voter}] retracted vote for [{f_votee}]",
  "VOTE":       "[{voter}] votes for [{votee}]",
  "MTARGET":    "[{actor}] prepares to kill [{target}]",
  "TARGET":     "You have targeted [{target}]",
  "NOTARGET":   "You have decided not to target anyone tonight",
  "REVEAL":     "[{actor}] is {role}",
  "REVEAL_REMINDER": "Remember, [{actor}] is {role}",
  "TIMER_DAY":  "Timer: nokill",
  "TIMER_NIGHT":"Timer: some slept through the night",
  "ELECT":      "[{target}] has been elected to be killed",
  "ELECT_NOKILL":"You have elected not to kill anyone",
  "ELECT_IDIOT": "... They were an IDIOT...",
  "ELECT_DAY"  :" The Day will continue!",
  "ELECT_STUN" :" All who voted for the IDIOT will be stunned tonight.",
  "KILL":       "[{target}] was killed by the mafia!",
  "KILL_FAIL_QUIET":  "It seems nobody died last night...",
  "VENGEANCE":  "[{actor}] takes [{target}] with them",
  "ELIMINATE" : "[{target}] was {role}",
  "ELIMINATE_ANON":"[{target}] has died",
  "CHARGE_DIE": "[{target}] has died",
  "CHARGE_KILLED": ", at the hands of [{aggressor}]",
  "DEATH":      "[{player}] was {role}",
  "STRIP":      "You were distracted...",
  "STUN":       "You are stunned until next morning",
  "SAVE":       "[{target}] was saved after being attacked by the mafia!",
  "SAVE_SECRET":"Somebody was saved after being attacked by the mafia!",
  "SAVE_DOC":   "You saved your patient!",
  "SAVE_SELF":  "You were saved!",
  "MILK":       "[{target}] received milk in the night.",
  "NO_MILK_SELF": "Ewww, please don't milk yourself in front of me",
  "INVESTIGATE":"[{target}] is {role}",
  "DAWN":       "Day dawns",
  "DAY":        "Pick someone to elect.",
  "NIGHT":      "Night falls",
  "NIGHT_OPTIONS":"Pick someone to target:\n",
  "DUSK":       "The sky darkens as their reddening eyes observe the crowd...",
  "DUSK_OPTIONS": "Pick someone who voted for you to kill:\n",
  "START":      "Start Game:\n",
  "WIN":   "{winning_team} Wins!",
  "REFOCUS": "Refocus {role} [{actor}] -> {new_role}, [{target}] -> [{aggressor}]",
  "REFOCUS_SELF": "Refocus {role} [{actor}] -> {new_role}, [{target}] -> self",
  "SURVIVOR_IDIOT_DIE": "{role} [{player}] died, killed by [{aggressor}]",
  "CONTRACT_WIN":"{role} [{player}] won! Charge: [{charge}]",
  "CONTRACT_LOSE":"{role} [{player}] lost! Charge: [{charge}]",
  "CONTRACT_RESULT":"{role} [{contractor}] {result}! Charge: [{charge}]",

  "UNKNOWN_REQ": "Unknown request, '{req_type}' in {chat_type} chat",
  "VOTE_ERROR": "/vote failed: {reason}",
  "MAIN_STATUS": "",
  "MAFIA_STATUS": "",
  "DM_STATUS": "",
  "TIMER_ERROR": "/timer failed: {reason}",
  "UNTIMER_ERROR": "/untimer failed: {reason}",
  "TIMER_REMINDER": "{minutes} minutes remaining.",
  "START_TIMER": "[{player_id}] started timer.",
  "ADD_TIME": "[{player_id}] added time to timer.",
  "CANCEL_TIMER": "[{player_id}] canceled timer.",
  "REMOVE_TIME": "[{player_id}] removed time from timer.",
  "MTARGET_ERROR": "/target failed: {reason}",
  "MOPTIONS_ERROR": "/options failed: {reason}",
  "TARGET_ERROR": "/target failed: {reason}",
  "OPTIONS_ERROR": "/options failed: {reason}",
  "REVEAL_ERROR": "/reveal failed: {reason}",
}

def dispVoteThresh(new_votee, former = False):
  former_str = "f_" if former else ""
  votee = former_str + "votee"
  if new_votee == NOTARGET:
    thresh = "{0.no_kill_thresh}"
    goal = 'for peace'
  else:
    thresh = "{thresh}"
    goal = 'to elect [{votee}]'.format(votee="{"+votee+"}")
  return "{num_votes}/{thresh} ".format(num_votes="{num_"+former_str+"voters}", thresh=thresh) + goal


def listMenu(players):
  ps = []
  c = 'A'
  for player in players:
    ps.append("{}: [{}]".format(c,player))
    c = chr(ord(c)+1)
  ps.append("{}: [NOTARGET]".format(c))
  return ps


def teamFromRole(role):
  if role in TOWN_ROLES:
    return "Town"
  if role in MAFIA_ROLES:
    return "Mafia"
  if role in ROGUE_ROLES:
    return "Rogue"


def dispRole(role, level="ON"):
  if level in ["ON","ROLE"]:
    return role
  elif level == "TEAM":
    m = teamFromRole(role)
    return m + " Aligned"
  elif level == "MAFIA":
    m = "Mafia" if teamFromRole(role)=="Mafia" else "Not Mafia"
    return m + " Aligned"
  else:
    return "[REDACTED]"


def makeRoleDict(roles):
  roleDict = {}
  for role in roles:
    if not role in roleDict:
      roleDict[role] = 0
    roleDict[role] += 1
  return roleDict


def dispRoleFromDict(roleDict):
  msgs = []
  for role in ALL_ROLES:
    if role in roleDict:
      msgs.append("{role}: {amt}".format(role=role, amt=roleDict[role]))
  return '\n'.join(msgs)


def dispTeamFromDict(roleDict, known_roles):
  Town = 0
  Mafia = 0
  Rogue = 0
  for role,n in roleDict.items():
    if role in TOWN_ROLES:
      Town += n
    elif role in MAFIA_ROLES:
      Mafia += n
    elif role in ROGUE_ROLES:
      Rogue += n
  if known_roles == "TEAM":
    if Rogue > 0:
      return "Town Aligned: {}\nMafia Aligned: {}\nRogue: {}\nTotal: {}".format(Town, Mafia, Rogue, Town+Mafia+Rogue)
    else:
      return "Town Aligned: {}\nMafia Aligned: {}\nTotal: {}".format(Town,Mafia, Town+Mafia)
  elif known_roles == "MAFIA":
    return "Mafia Aligned: {}\nTotal: {}".format(Mafia, Town+Mafia+Rogue)
  else:
    raise ValueError(str(known_roles) + " wasn't TEAM or MAFIA")

def dispKnownRoles(roleDict, known_roles):
  if known_roles == "ROLE":
    return dispRoleFromDict(roleDict)
  elif known_roles in ("TEAM", "MAFIA"):
    return dispTeamFromDict(roleDict, known_roles)
  elif known_roles == "OFF":
    return "Players: {}".format(len(roleDict))

class MEvent:

  def read(self, mstate):
    pass # This phase is for reading data from mstate and doing initial calculations

  def msg(self, cast_main, cast_mafia, send_dm):
    pass

  def write(self, mstate):
    pass

  def next(self, pushEvent):
    pass

class START(MEvent):
  def __init__(self, ids, roles, contracts):
    self.ids = ids
    self.roles = roles
    self.contracts = contracts

  def read(self, mstate):
    self.known_roles = mstate.rules[known_roles]
    self.start_night = mstate.rules[start_night]

  def msg(self, cast_main, cast_mafia, send_dm):
    roleDict = makeRoleDict(self.roles)
    msg = default_resp_lib["START"]
    msg += dispKnownRoles(roleDict, self.known_roles)

    cast_main(msg)

  def write(self, mstate):
    for i,role in zip(self.ids, self.roles):
      player = MPlayer(i,role)
      mstate.players[i] = player
    self.phase = MPhase.DAY
    if (self.start_night == "ON" or 
      (self.start_night == "ODD" and len(self.ids) % 2 == 1) or
      (self.start_night == "EVEN" and len(self.ids) % 2 == 0)):
      self.phase = MPhase.NIGHT
    mstate.phase = self.phase
    mstate.day = 1

    mstate.contracts = self.contracts

  def next(self, push):
    if self.phase == MPhase.DAY:
      push(DAY())
    elif self.phase == MPhase.NIGHT:
      push(NIGHT())
      
class TIMER(MEvent):

  def read(self, mstate):
    self.phase = mstate.phase

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.phase == MPhase.DAY or self.phase == MPhase.DUSK:
      msg = default_resp_lib["TIMER_DAY"]
    elif self.phase == MPhase.NIGHT:
      msg = default_resp_lib["TIMER_NIGHT"]
    cast_main(msg)
    
  def next(self, push):
    if self.phase == MPhase.DAY or self.phase == MPhase.DUSK:
      push(NIGHT())
      return
    if self.phase == MPhase.NIGHT:
      push(DAWN())

class NIGHT(MEvent):
  def read(self, mstate):
    self.players = mstate.players.keys()
    self.targeting_players = [p for p in mstate.players if mstate.players[p].role in TARGETING_ROLES]
    self.stunned = mstate.stunned

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["NIGHT"]
    cast_main(msg)
    for targeting_player in self.targeting_players:
      if not targeting_player in self.stunned:
        msg = default_resp_lib["NIGHT_OPTIONS"]
        msg += "\n".join(listMenu(self.players))
        send_dm(msg, targeting_player)
    msg = default_resp_lib["NIGHT_OPTIONS"]
    msg += "\n".join(listMenu(self.players))
    cast_mafia(msg)


  def write(self, mstate):
    mstate.phase = MPhase.NIGHT
    for player in mstate.players.values():
      player.vote = None
    mstate.stripped = []
    mstate.vengeance = []

class MTARGET(MEvent):

  def __init__(self, actor, target):
    self.actor = actor
    self.target = target

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["MTARGET"].format(actor=self.actor, target=self.target)
    cast_mafia(msg)

  def write(self, mstate):
    mstate.mafia_target = self.target
    mstate.mafia_targeter = self.actor
    self.finished = (mstate.mafia_target != None) and all(
        [p.target != None for p in mstate.players.values() if p.role in TARGETING_ROLES])

  def next(self, push):
    if self.finished:
      push(DAWN())

class TARGET(MEvent):
  
  def __init__(self, actor, target):
    self.actor = actor
    self.target = target

  def read(self, mstate):
    # asserts
    self.phase = mstate.phase
    
  def msg(self, cast_main, cast_mafia, send_dm):
    if self.target == NOTARGET:
      msg = default_resp_lib["NOTARGET"]
    else:
      msg = default_resp_lib["TARGET"].format(target=self.target)
    send_dm(msg, self.actor)

  def write(self, mstate):
    actor = mstate.players[self.actor]
    actor.target = self.target
    
    if self.phase == MPhase.NIGHT:
      self.finished = (mstate.mafia_target != None) and all(
        [p.target != None for p in mstate.players.values() if p.role in TARGETING_ROLES])
    elif self.phase == MPhase.DUSK:
      idiot = mstate.vengeance["idiot"]
      self.finished = mstate.players[idiot].target != None

  def next(self, push):
    if self.phase == MPhase.NIGHT:
      if self.finished:
        push(DAWN())
    elif self.phase == MPhase.DUSK:
      push(VENGEANCE(self.actor,self.target))

class DAWN(MEvent):
  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["DAWN"]
    cast_main(msg)

  def write(self, mstate):
    # TODO: Calculate the entire dawn!
    # determine success/failure of each event

    event_list = []

    for stripper_id in [p for p in mstate.players if mstate.players[p].role == "STRIPPER"]:
      stripper = mstate.players[stripper_id]
      if not stripper.target in (NOTARGET, None):
        mstate.stripped.append(stripper.target)
        event_list.append(STRIP(stripper_id, stripper.target))
    
    target_saved = False
    if not mstate.mafia_target in (NOTARGET, None):
      for doctor_id in [p for p in mstate.players if mstate.players[p].role == "DOCTOR"]:
        doctor = mstate.players[doctor_id]
        if not doctor.target in (NOTARGET, None):
          success = doctor.target == mstate.mafia_target
          stripped = doctor_id in mstate.stripped
          event_list.append(SAVE(doctor_id, doctor.target, stripped, success))
          if success and not stripped:
            target_saved = True
    event_list.append(KILL(mstate.mafia_targeter, mstate.mafia_target, not target_saved))

    # TODO: add checks in MILK and INVESTIGATE to ensure they aren't ded
    for milky_id in [p for p in mstate.players if mstate.players[p].role == "MILKY"]:
      milky = mstate.players[milky_id]
      if not milky.target in (NOTARGET, None):
        stripped = milky_id in mstate.stripped
        success = (not (milky_id == mstate.mafia_target and not target_saved) and 
          not (milky.target == mstate.mafia_target and not target_saved))
        event_list.append(MILK(milky_id, milky.target, stripped, success))
    
    for cop_id in [p for p in mstate.players if mstate.players[p].role == "COP"]:
      cop = mstate.players[cop_id]
      if not cop.target in (NOTARGET,None):
        stripped = cop_id in mstate.stripped
        success = (not (cop_id == mstate.mafia_target and not target_saved) and 
          not (cop.target == mstate.mafia_target and not target_saved))
        event_list.append(INVESTIGATE(cop_id, cop.target, stripped, success))

    event_list.append(DAY())
    self.event_list = event_list
    # TODO: randomize order for milks?

  def next(self, push):
    push(self.event_list)

class STRIP(MEvent):
  def __init__(self, actor, target):
    self.actor = actor
    self.target = target
  
  def read(self, mstate):
    self.know_if_stripped = mstate.rules[know_if_stripped]
    self.target_role = mstate.players[self.target].role

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.know_if_stripped == "ON":
      msg = "You were distracted..."
      send_dm(msg, self.target)
    elif self.know_if_stripped == "TARGET":
      if self.target_role in TARGETING_ROLES or self.target_role == "CELEB":
        msg = "You were distracted..."
        send_dm(msg, self.target)     

class SAVE(MEvent):
  def __init__(self, actor, target, stripped, success):
    self.actor = actor
    self.target = target
    self.stripped = stripped
    self.success = success

  def read(self, mstate):
    self.know_if_stripped = mstate.rules[know_if_stripped]
    self.know_if_saved = mstate.rules[know_if_saved]
    self.know_if_saved_doc = mstate.rules[know_if_saved_doc]
    self.know_if_saved_self = mstate.rules[know_if_saved_self]

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.stripped:
      if self.know_if_stripped == "USEFUL":
        msg = default_resp_lib["STRIP"]
        send_dm(msg, self.actor)
    elif self.success:
      if self.know_if_saved == "SAVED":
        msg = default_resp_lib["SAVE"].format(target=self.target)
        cast_main(msg)
      elif self.know_if_saved == "SECRET":
        msg = default_resp_lib["SAVE_SECRET"]
        cast_main(msg)
      elif self.know_if_saved == "OFF":
        msg = default_resp_lib["KILL_FAIL_QUIET"]
        cast_main(msg)
      if self.know_if_saved_doc == "ON":
        msg = default_resp_lib["SAVE_DOC"]
        send_dm(msg, self.actor)
      if self.know_if_saved_self == "ON":
        msg = default_resp_lib["SAVE_SELF"]
        send_dm(msg, self.target)

class KILL(MEvent):
  def __init__(self, actor, target, success):
    self.actor = actor
    self.target = target
    self.success = success

  def read(self, mstate):
    if self.success and not self.target in (NOTARGET, None):
      self.role = mstate.players[self.target].role

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.target in (NOTARGET, None):
      msg = default_resp_lib["KILL_FAIL_QUIET"]
      cast_main(msg)
    elif self.success:
      msg = default_resp_lib["KILL"].format(target=self.target)
      cast_main(msg)

  def write(self, mstate):
    mstate.mafia_target = None
    mstate.mafia_targeter = None

  def next(self, push):
    if self.success and not self.target in (NOTARGET, None):
      push(ELIMINATE(self.actor, self.target))

class MILK(MEvent):
  def __init__(self, actor, target, stripped, success):
    self.actor = actor
    self.target = target
    self.stripped = stripped
    self.success = success
  
  def read(self, mstate):
    self.know_if_stripped = mstate.rules[know_if_stripped]

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.success:
      if self.stripped:
        if self.know_if_stripped == "USEFUL":
          msg = default_resp_lib["STRIP"]
          send_dm(msg, self.actor)
      else:
        msg = default_resp_lib["MILK"].format(target=self.target)
        cast_main(msg)

class INVESTIGATE(MEvent):
  def __init__(self, actor, target, stripped, success):
    self.actor = actor
    self.target = target
    self.stripped = stripped
    self.success = success

  def read(self, mstate):
    if self.success:
      self.role = mstate.players[self.target].role
    self.cop_strength = mstate.rules[cop_strength]
    self.know_if_stripped = mstate.rules[know_if_stripped]

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.success:
      if self.stripped:
        if self.know_if_stripped == "USEFUL":
          msg = default_resp_lib["STRIP"]
          send_dm(msg, self.actor)
      else:
        reveal = self.role
        if reveal == "GODFATHER":
          reveal = "TOWN"
        elif reveal == "MILLER":
          reveal = "MAFIA"
        msg = default_resp_lib["INVESTIGATE"].format(target=self.target,role=self.role)
        send_dm(msg, self.actor)

class DAY(MEvent):

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["DAY"]
    cast_main(msg)

  def write(self, mstate):
    mstate.mafia_target = None
    mstate.mafia_targeter = None
    for player in mstate.players.values():
      if not player.role in CONTRACT_ROLES:
        player.target = None
    mstate.stunned = []
    mstate.day += 1
    mstate.phase = MPhase.DAY

class REVEAL(MEvent):
  
  def __init__(self,actor):
    self.actor = actor

  def read(self, mstate):
    self.stripped = mstate.checkStripped(self.actor)
    self.revealed = mstate.checkRevealed(self.actor)
    self.role = mstate.players[self.actor].role

  def msg(self, cast_main, cast_mafia, send_dm):
    if not self.stripped:
      if not self.revealed:
        msg = default_resp_lib["REVEAL"].format(actor=self.actor,role=self.role)
      else:
        msg = default_resp_lib["REVEAL_REMINDER"].format(actor=self.actor,role=self.role)
      cast_main(msg)
    else:
      msg = default_resp_lib["STRIP"]
      send_dm(msg, self.actor)

  def write(self, mstate):
    if not self.stripped:
      if not self.actor in mstate.revealed:
        mstate.revealed.append(self.actor)

class VOTE(MEvent):

  def __init__(self, voter, votee):
    self.voter = voter
    self.votee = votee

  def read(self, mstate):
    # TODO: include error checking?
    players = mstate.players
    voter = players[self.voter]
    self.f_votee = voter.vote

    if self.votee in players or self.votee in (NOTARGET, None):
      voter.vote = self.votee

    self.num_voters = len([v for v in players if players[v].vote == self.votee])
    self.num_f_voters = len([v for v in players if players[v].vote == self.f_votee])
    self.num_players = len(players)
    self.thresh = int(self.num_players/2) + 1
    self.no_kill_thresh = self.num_players - self.thresh + 1

  def msg(self, cast_main, cast_mafia, send_dm):

    def dispVoteThresh(votee, thresh, no_kill_thresh, num_voters):
      if votee == NOTARGET:
        used_thresh = no_kill_thresh
        goal = " for peace"
      else:
        used_thresh = thresh
        goal = " to elect [{}]".format(votee)
      return "{}/{}".format(num_voters, used_thresh) + goal

    msg = default_resp_lib['VOTE'].format(voter=self.voter, votee=self.votee)
    if self.votee == None:
      msg = default_resp_lib['VOTE_RETRACT'].format(voter=self.voter,f_votee=self.f_votee)
    else:
      # Add vote thresh info
      msg += ", " + dispVoteThresh(self.votee, self.thresh, self.no_kill_thresh, self.num_voters)
    if self.f_votee != None:
      msg += ", " + dispVoteThresh(self.f_votee, self.thresh, self.no_kill_thresh, self.num_f_voters)
      
    cast_main(msg)

  def next(self, pushEvent):
    if self.votee == None:
      return

    if ((self.votee == NOTARGET and self.num_voters >= self.no_kill_thresh) or
      (self.votee != NOTARGET and self.num_voters >= self.thresh)):
      pushEvent(ELECT(self.voter, self.votee))
      return

class ELECT(MEvent):
  def __init__(self, actor, target):
    self.actor = actor
    self.target = target

  def read(self, mstate):
    if not self.target in (NOTARGET,None):
      self.role = mstate.players[self.target].role
    self.idiot_vengeance = mstate.rules[idiot_vengeance]
    self.stunned = mstate.stunned
    self.players = mstate.players

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["ELECT"].format(target=self.target)
    if self.idiot_vengeance != "OFF":
      if not self.target in (NOTARGET, None) and self.role == "IDIOT":
        msg += default_resp_lib["ELECT_IDIOT"]
        if self.idiot_vengeance == "DAY":
          msg += default_resp_lib["ELECT_DAY"]
        elif self.idiot_vengeance == "STUN":
          msg += default_resp_lib["ELECT_STUN"].format(target=self.target)
          for player in self.players:
            if player in self.stunned:
              send_dm(default_resp_lib["STUN"], player)
    cast_main(msg)

  def write(self, mstate):
    if not self.target in (NOTARGET, None):
      player = mstate.players[self.target]
      if player.role == "IDIOT":
        # Set the IDIOT's victory condition to true
        (role,charge,_) = mstate.contracts[self.target]
        mstate.contracts[self.target] = (role,charge,True)
        self.venges = [p_id for p_id in mstate.players if (mstate.players[p_id].vote == self.target and p_id != self.target)]
        mstate.vengeance = {
          'venges': self.venges,
          'final_vote': self.actor,
          'idiot': self.target}
        if self.idiot_vengeance == "DAY":
          # Reset votes
          for player in mstate.players.values():
            player.vote = None
        elif self.idiot_vengeance == "STUN":
          mstate.stunned = self.venges

  def next(self, push):
    event_list = []
    if not self.target in (NOTARGET, None):
      if self.role == "IDIOT":
        if self.idiot_vengeance == "KILL":
          push(DUSK(self.target))
          return
        elif self.idiot_vengeance == "WIN":
          push(WIN("IDIOT"))
          return
        elif self.idiot_vengeance == "DAY":
          push(ELIMINATE(self.actor, self.target))
          return
      else:
        event_list.append(ELIMINATE(self.actor, self.target))
    event_list.append(NIGHT())
    push(event_list)

class DUSK(MEvent):
  def __init__(self, idiot):
    self.idiot = idiot

  def read(self, mstate):
    self.venges = mstate.vengeance['venges']

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["DUSK"]
    cast_main(msg)
    msg = default_resp_lib["DUSK_OPTIONS"]
    msg += "\n".join(listMenu(self.venges))
    send_dm(msg, self.idiot)

  def write(self, mstate):
    mstate.phase = MPhase.DUSK

class VENGEANCE(MEvent):
  def __init__(self, actor, target):
    self.actor = actor
    self.target = target

  def read(self, mstate):
    if not self.target in (NOTARGET, None):
      self.role = mstate.players[self.target].role
    self.vengeance = mstate.vengeance

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["VENGEANCE"].format(actor=self.actor, target=self.target)
    cast_main(msg)

  def next(self, push):
    event_list = []
    if not self.target in (NOTARGET, None):
      event_list.append(ELIMINATE(self.actor, self.target))
    event_list.append(ELIMINATE(self.vengeance["final_vote"], self.vengeance["idiot"]))
    push(event_list)

class ELIMINATE(MEvent):
  def __init__(self, actor, target):
    self.actor = actor
    self.target = target

  def read(self, mstate):
    self.role = mstate.players[self.target].role
    self.reveal_on_death = mstate.rules[reveal_on_death]

  def msg(self, cast_main, cast_mafia, send_dm):
    reveal = dispRole(self.role, self.reveal_on_death)
    msg = default_resp_lib["ELIMINATE"].format(target=self.target,role=reveal)
    cast_main(msg)

  def write(self, mstate):
    del mstate.players[self.target]
    
    self.relevant_contractors = []
    for (p,(role,charge,_)) in mstate.contracts.items():
      if charge == self.target:
        self.relevant_contractors.append((p,role))
    self.num_players = len(mstate.players)
    self.num_mafia = len([1 for p in mstate.players.values() if p.role in MAFIA_ROLES])

  def next(self, push):
    event_list = []
    for (p,role) in self.relevant_contractors:
      event_list.append(CHARGE_DIE(p, self.target, self.actor, role))
    if self.num_mafia == 0:
      event_list.append(WIN("Town"))
    elif self.num_mafia >= (self.num_players+1) // 2:
      event_list.append(WIN("Mafia"))
    push(event_list)

class CHARGE_DIE(MEvent):
  def __init__(self, actor, target, aggressor, role):
    self.actor = actor
    self.target = target
    self.aggressor = aggressor
    self.role = role

  def read(self, mstate):
    self.charge_refocus_guard = mstate.rules[charge_refocus_guard]
    self.charge_refocus_agent = mstate.rules[charge_refocus_agent]

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["CHARGE_DIE"].format(target=self.target, aggressor=self.aggressor)
    send_dm(msg, self.actor)

  def write(self, mstate):
    (self.role,charge,success) = mstate.contracts[self.actor]
    if self.role == "AGENT":
      success = True
    elif self.role in ("GUARD", "SURVIVOR"):
      success = False

    mstate.contracts[self.actor] = (self.role,charge,success)

  def next(self, push):
    if ((self.role == "GUARD" and self.charge_refocus_guard) or
      (self.role == "AGENT" and self.charge_refocus_agent)):
      push(REFOCUS(self.actor, self.target, self.aggressor, self.role))

class REFOCUS(MEvent):
  def __init__(self, actor, target, aggressor, role):
    self.actor = actor
    self.target = target
    self.aggressor = aggressor
    self.role = role

  def read(self, mstate):
    if self.actor == self.aggressor:
      if self.role == "GUARD":
        self.new_role = "IDIOT"
      elif self.role == "AGENT":
        self.new_role = "SURVIVOR"
    else:
      if self.role == "GUARD":
        self.new_role = "AGENT"
      elif self.role == "AGENT":
        self.new_role = "GUARD"

  def msg(self, cast_main, cast_mafia, send_dm):
    if self.actor == self.aggressor:
      msg = default_resp_lib["REFOCUS_SELF"].format(
        role=self.role, actor=self.actor, new_role=self.new_role, target=self.target)
    else:
      msg = default_resp_lib["REFOCUS"].format(
        role=self.role, actor=self.actor, new_role=self.new_role,
        target=self.target, aggressor=self.aggressor
      )
    send_dm(msg, self.actor)

  def write(self, mstate):
    if self.actor == self.aggressor or not self.aggressor in mstate.players:
      new_charge = self.actor
    else:
      new_charge = self.aggressor
    player = mstate.players[self.actor]
    player.role = self.new_role
    player.target = new_charge

    mstate.contracts[self.actor] = (
      player.role, player.target, player.role in ("SURVIVOR","GUARD")
    )

class WIN(MEvent):
  def __init__(self, winning_team):
    self.winning_team = winning_team

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["WIN"].format(winning_team=self.winning_team)
    cast_main(msg)

  def write(self, mstate):
    self.contracts = mstate.contracts

  def next(self, push):
    event_list = []
    for contractor in self.contracts:
      (role, charge, success) = self.contracts[contractor]
      event_list.append(CONTRACT_RESULT(contractor, role, charge, success))
    event_list.append(END(self.winning_team))
    push(event_list)
      

class CONTRACT_RESULT(MEvent):
  def __init__(self, contractor, role, charge, success):
    self.contractor = contractor
    self.role = role
    self.charge = charge
    self.success = success

  def msg(self, cast_main, cast_mafia, send_dm):
    msg = default_resp_lib["CONTRACT_RESULT"].format(
      role=self.role, contractor=self.contractor,
      result="Won" if self.success else "Lost",
      charge=self.charge
    )
    cast_main(msg)

class END(MEvent):
  def __init__(self, winning_team):
    self.winning_team = winning_team

  def next(self, push):
    # TODO: implement end state stuff
    raise EndGameException("End Game {}".format(self.winning_team))
