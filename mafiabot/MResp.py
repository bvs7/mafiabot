from typing import Dict, Callable, Any, NewType
from enum import Enum, auto

from .MPlayer import ALL_ROLES, TOWN_ROLES, MAFIA_ROLES, ROGUE_ROLES
from .MEx import NOTARGET

# Resp must implement all of these Response types?
class MRespType(Enum):
  VOTE_RETRACT = auto()
  VOTE = auto()
  MTARGET = auto()
  TARGET = auto()
  REVEAL = auto()
  TIMER_DAY = auto()
  TIMER_NIGHT = auto()
  ELECT = auto()
  ELECT_NOKILL = auto()
  ELECT_IDIOT = auto()
  KILL = auto()
  DEATH = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  NO_MILK_SELF = auto()
  INVESTIGATE = auto()
  DAY_PREAMBLE = auto()
  DAY = auto()
  NIGHT = auto()
  NIGHT_OPTIONS = auto()
  DUSK = auto()
  DUSK_OPTIONS = auto()
  IDIOT_KILL = auto()
  START = auto()
  TOWN_WIN = auto()
  MAFIA_WIN = auto()
  CHARGE_REFOCUS = auto()
  CHARGE_REFOCUS_SELF = auto()
  SURVIVOR_IDIOT_DIE = auto()
  CONTRACT_WIN = auto()
  CONTRACT_LOSE = auto()

  # External MRespTypes (from Handler)
  UNKNOWN_REQ = auto()
  VOTE_ERROR = auto()
  MAIN_STATUS = auto()
  MAFIA_STATUS = auto()
  DM_STATUS = auto()
  TIMER_ERROR = auto()
  UNTIMER_ERROR = auto()
  TIMER_REMINDER = auto()
  START_TIMER = auto()
  ADD_TIME = auto()
  CANCEL_TIMER = auto()
  REMOVE_TIME = auto()
  MTARGET_ERROR = auto()
  MOPTIONS_ERROR = auto()
  TARGET_ERROR = auto()
  OPTIONS_ERROR = auto()
  REVEAL_ERROR = auto()

  # Lobby Resps


default_resp_lib = {
  MRespType.VOTE_RETRACT: "[{voter}] retracted vote for [{former_votee}]",
  MRespType.VOTE:       "[{voter}] votes for [{votee}]",
  MRespType.MTARGET:    "[{actor}] prepares to kill [{target}]",
  MRespType.TARGET:     "You have targeted [{target}]",
  MRespType.REVEAL:     "Reveal: [{actor}]",
  MRespType.TIMER_DAY:  "Timer: nokill",
  MRespType.TIMER_NIGHT:"Timer: some slept through the night",
  MRespType.ELECT:      "[{target}] has been elected to be killed",
  MRespType.ELECT_NOKILL:"You have elected not to kill anyone",
  MRespType.ELECT_IDIOT: "... They were an IDIOT...",
  MRespType.KILL:       "[{target}] was killed by the mafia!",
  MRespType.DEATH:      "[{player}] was {role}",
  MRespType.STRIP:      "You were distracted...",
  MRespType.SAVE:       "[{target}] was saved after being attacked by the mafia!",
  MRespType.MILK:       "[{target}] received milk in the night.",
  MRespType.NO_MILK_SELF: "Ewww, please don't milk yourself in front of me",
  MRespType.INVESTIGATE:"[{target}] is {role}",
  MRespType.DAY_PREAMBLE:"Day dawns",
  MRespType.DAY:        "Pick someone to elect.",
  MRespType.NIGHT:      "Night falls",
  MRespType.NIGHT_OPTIONS:"Pick someone to {act}:\n",
  MRespType.DUSK:       "Oops! [{idiot}] was IDIOT. The sky darkens as their reddening eyes observe the crowd...",
  MRespType.DUSK_OPTIONS: "Pick someone who voted for you to kill:\n",
  MRespType.IDIOT_KILL: "[{actor}] kills [{target}] before the crowd can subdue them",
  MRespType.START:      "Start Game:",
  MRespType.TOWN_WIN:   "Town Wins",
  MRespType.MAFIA_WIN:  "Mafia Wins",
  MRespType.CHARGE_REFOCUS: "Refocus {role} [{player}] -> {new_role}, [{charge}] -> [{aggressor}]",
  MRespType.CHARGE_REFOCUS_SELF: "Refocus {role} [{player}] -> {new_role}, [{target}] -> self",
  MRespType.SURVIVOR_IDIOT_DIE: "{role} [{player}] died, killed by [{aggressor}]",
  MRespType.CONTRACT_WIN:"{role} [{player}] won! Charge: [{charge}]",
  MRespType.CONTRACT_LOSE:"{role} [{player}] lost! Charge: [{charge}]",

  MRespType.UNKNOWN_REQ: "Unknown request, '{req_type}' in {chat_type} chat",
  MRespType.VOTE_ERROR: "/vote failed: {reason}",
  MRespType.MAIN_STATUS: "",
  MRespType.MAFIA_STATUS : "",
  MRespType.DM_STATUS : "",
  MRespType.TIMER_ERROR : "/timer failed: {reason}",
  MRespType.UNTIMER_ERROR : "/untimer failed: {reason}",
  MRespType.TIMER_REMINDER : "{minutes} minutes remaining.",
  MRespType.START_TIMER : "[{player_id}] started timer.",
  MRespType.ADD_TIME : "[{player_id}] added time to timer.",
  MRespType.CANCEL_TIMER : "[{player_id}] canceled timer.",
  MRespType.REMOVE_TIME : "[{player_id}] removed time from timer.",
  MRespType.MTARGET_ERROR: "/target failed: {reason}",
  MRespType.MOPTIONS_ERROR: "/options failed: {reason}",
  MRespType.TARGET_ERROR: "/target failed: {reason}",
  MRespType.OPTIONS_ERROR: "/options failed: {reason}",
  MRespType.REVEAL_ERROR: "/reveal failed: {reason}",
}

class MResp:

  def resp(self, typ : MRespType, **kwargs) -> None:
    # This function should be overloaded by a subclass?

    try:
      print(default_resp_lib[typ].format(**kwargs))
    except Exception as e:
      print(e)
      raise(e)

  @staticmethod
  def dispVotes(players):
    num_players = len(players)
    thresh = int(num_players/2) + 1
    no_kill_thresh = num_players - thresh + 1
    msgs =[]
    rev_vote_dict = {}
    for p in players:
      rev_vote_dict[p] = [voter for voter in players if players[voter].vote == p]
    for p,voters in rev_vote_dict.items():
      if len(voters) > 0:
        msg = "[{p}] ({n}/{thresh}): ".format(p=p, n=len(voters), thresh=thresh)
        for voter in voters:
          msg += "\n  [{voter}]".format(voter=voter)
        msgs.append(msg)
    no_target_votes = [voter for voter in players if players[voter].vote == NOTARGET]
    if len(no_target_votes) > 0:
      msg = "[{p}] ({n}/{nk_thresh}): ".format(p=NOTARGET, n=len(no_target_votes), nk_thresh=no_kill_thresh)
      for voter in no_target_votes:
        msg += "\n  [{voter}]".format(voter=voter)
      msgs.append(msg)
    return "\n".join(msgs)

  @staticmethod
  def teamFromRole(role):
    if role in TOWN_ROLES:
      return "Town"
    if role in MAFIA_ROLES:
      return "Mafia"
    if role in ROGUE_ROLES:
      return "Rogue"

  @staticmethod
  def dispRole(role, level):
    if level in ["ON","ROLE"]:
      return role
    elif level == "TEAM":
      m = MResp.teamFromRole(role)
      return m + " Aligned"
    elif level == "MAFIA":
      m = "Mafia" if MResp.teamFromRole(role)=="Mafia" else "Not Mafia"
      return m + " Aligned"
    else:
      return "[REDACTED]"

  @staticmethod
  def makeRoleDict(roles):
    roleDict = {}
    for role in roles:
      if not role in roleDict:
        roleDict[role] = 0
      roleDict[role] += 1
    return roleDict

  @staticmethod
  def dispRoleFromDict(roleDict):
    msgs = []
    for role in ALL_ROLES:
      if role in roleDict:
        msgs.append("{role}: {amt}".format(role=role, amt=roleDict[role]))
    return '\n'.join(msgs)
  
  @staticmethod
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

  @staticmethod
  def dispStatus(mstate, known_roles, reveal_on_death):
    msg = "Game #{game_id}, {phase} {day}".format(game_id=mstate.id, phase=mstate.phase, day=mstate.day)
    if mstate.phase == "Day":
      msg += ":\n" + MResp.dispVotes(mstate.players)
    elif mstate.phase == "Dusk":
      msg += ":\n[{}] is seeking revenge against :\n  [".format(
        mstate.venger) + "]\n  [".join(mstate.venges) + "]"
    players = mstate.players
    msg += '\n'
    roleDict = MResp.makeRoleDict([p.role for p in players.values()])
    if known_roles == "ROLE" and reveal_on_death == "ROLE":
      msg += MResp.dispRoleFromDict(roleDict)
    elif known_roles in ['ROLE','TEAM'] and reveal_on_death in ['ROLE','TEAM']:
      msg += MResp.dispTeamFromDict(roleDict, "TEAM")
    elif known_roles in ['ROLE','TEAM', 'MAIFA'] and reveal_on_death in ['ROLE','TEAM','MAFIA']:
      msg += MResp.dispTeamFromDict(roleDict, 'MAFIA')
    elif known_roles == 'OFF' or reveal_on_death == 'OFF':
      msg += "Players: {}".format(len(players))
    else:
      NotImplementedError("known_roles or reveal_on_death unknown value: {} | {}".format(known_roles, reveal_on_death))
    return msg

class TestMResp(MResp):
  
  def __init__(self, check : Callable [[MRespType,Dict[str,Any]],bool]):
    # Calls resp on this fn to compare to a line of others
    self.check : Callable [[MRespType,Dict[str,Any]],bool] = check

  def resp(self, typ : MRespType, **kwargs) -> None:
    if not self.check(typ, kwargs):
      raise TestMRespException()

class TestMRespException(Exception):
  pass