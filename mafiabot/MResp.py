from typing import Dict, Callable, Any, NewType
from enum import Enum, auto

from .MPlayer import ALL_ROLES, TOWN_ROLES, MAFIA_ROLES, ROGUE_ROLES

# Resp must implement all of these Response types?
class MRespType(Enum):
  VOTE_RETRACT = auto()
  VOTE_NOKILL = auto()
  VOTE_PLAYER = auto()
  MTARGET = auto()
  TARGET = auto()
  REVEAL = auto()
  TIMER_DAY = auto()
  TIMER_NIGHT = auto()
  ELECT = auto()
  KILL = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  INVESTIGATE = auto()
  DAY_PREAMBLE = auto()
  DAY = auto()
  NIGHT = auto()
  START = auto()
  TOWN_WIN = auto()
  MAFIA_WIN = auto()
  CHARGE_REFOCUS = auto()
  CHARGE_REFOCUS_SELF = auto()
  SURVIVOR_IDIOT_DIE = auto()
  CONTRACT_WIN = auto()
  CONTRACT_LOSE = auto()


default_resp_lib = {
  MRespType.VOTE_RETRACT: "[{voter}] Retracted Vote",
  MRespType.VOTE_NOKILL: "Vote nokill: [{voter}], {remain} more for peace.",
  MRespType.VOTE_PLAYER: "Vote: [{voter}] -> [{votee}], {remain} more to elect.",
  MRespType.MTARGET:    "Mafia Target: [{target}]",
  MRespType.TARGET:     "Target: [{actor}] -> [{target}]",
  MRespType.REVEAL:     "Reveal: [{actor}]",
  MRespType.TIMER_DAY:  "Timer: nokill",
  MRespType.TIMER_NIGHT:"Timer: some slept through the night",
  MRespType.ELECT:      "Elect: [{target}]",
  MRespType.KILL:       "Kill: [{target}], success: {success}",
  MRespType.STRIP:      "Strip: [{actor}] -> [{target}], useful: {useful}",
  MRespType.SAVE:       "Save: [{actor}] -> [{target}], blocked: {blocked}, useful: {useful}",
  MRespType.MILK:       "Milk: [{actor}] -> [{target}], blocked: {blocked}, sniped: {sniped}",
  MRespType.INVESTIGATE:"Investigate: [{actor}] -> [{target}], blocked: {blocked}, sniped: {sniped}",
  MRespType.DAY_PREAMBLE:"",
  MRespType.DAY:        "",
  MRespType.NIGHT:      "Night",
  MRespType.START:      "Start Game:",
  MRespType.TOWN_WIN:   "Town Wins",
  MRespType.MAFIA_WIN:  "Mafia Wins",
  MRespType.CHARGE_REFOCUS: "Refocus {role} [{player}] -> {new_role}, [{charge}] -> [{aggressor}]",
  MRespType.CHARGE_REFOCUS_SELF: "Refocus {role} [{player}] -> {new_role}, [{target}] -> self",
  MRespType.SURVIVOR_IDIOT_DIE: "{role} [{player}] died, killed by [{aggressor}]",
  MRespType.CONTRACT_WIN:"{role} [{player}] won! Charge: [{charge}]",
  MRespType.CONTRACT_LOSE:"{role} [{player}] lost! Charge: [{charge}]",
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
        msgs.append("{role}: {amt}\n".format(role=role, amt=roleDict(role)))
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

class TestMResp(MResp):
  
  def __init__(self, check : Callable [[MRespType,Dict[str,Any]],bool]):
    # Calls resp on this fn to compare to a line of others
    self.check : Callable [[MRespType,Dict[str,Any]],bool] = check

  def resp(self, typ : MRespType, **kwargs) -> None:
    if not self.check(typ, kwargs):
      raise TestMRespException()

class TestMRespException(Exception):
  pass