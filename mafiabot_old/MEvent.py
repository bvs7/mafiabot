## Define MEvents

# MEvents are critical state changing events in the mafia game that should be
#  recorded. There are certain events that are exterior accessible, and others
#  that can only be generated from within MState. (Ex: Night event, going to
#  night is generated when votes are tallied or)

# TODO: Want an optional import of interface to define player variables
from typing import Dict, Any, NewType, Optional, List
from enum import Enum, auto

from .MEx import MPlayerID

class MEventType(Enum):
  # Events that can be generated externally by MHandler
  VOTE = auto()
  MTARGET = auto()
  TARGET = auto()
  REVEAL = auto()
  TIMER = auto()
  # Events that are generated internally by MState
  ELECT = auto()
  KILL = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  INVESTIGATE = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()
  START = auto()
  TOWN_WIN = auto()
  MAFIA_WIN = auto()
  AGENT_REFOCUS = auto()
  GUARD_REFOCUS = auto()
  SURVIVOR_DIE = auto()
  IDIOT_DIE = auto()
  CHARGE_DIE = auto()
  CONTRACT_RESULT = auto()

class MEvent:
  
  def __init__(self, typ : MEventType, data : Dict[str, Any] = {}):
    self.type = typ

    self.data = data

  def __getattr__(self, name):
    try:
      return self.data[name]
    except KeyError:
      raise AttributeError

class MEventC:

  # Static contructors for extrenal Events #
  @staticmethod
  def vote(voter : MPlayerID, votee : Optional[MPlayerID]):
    return MEvent(MEventType.VOTE, {'voter':voter, 'votee':votee})

  @staticmethod
  def mtarget(actor : MPlayerID, target : Optional[MPlayerID]):
    return MEvent(MEventType.MTARGET, {'actor':actor, 'target':target})

  @staticmethod
  def target(actor : MPlayerID, target : Optional[MPlayerID]):
    return MEvent(MEventType.TARGET, {'actor':actor, 'target':target})

  @staticmethod
  def reveal(actor : MPlayerID):
    return MEvent(MEventType.REVEAL, {'actor':actor})

  @staticmethod
  def timer():
    return MEvent(MEventType.TIMER)

  # Static constructors for internal Events #
  @staticmethod
  def elect(actor : MPlayerID, target : MPlayerID):
    return MEvent(MEventType.ELECT, {'actor':actor, 'target':target})

  @staticmethod
  def kill(actor : MPlayerID, target : MPlayerID, success : bool):
    return MEvent(MEventType.KILL, {'actor':actor, 'target':target, 'success':success})

  @staticmethod
  def strip(actor : MPlayerID, target : MPlayerID, role : str, useful : bool):
    return MEvent(MEventType.STRIP, {'actor':actor, 'target':target, 'role':role, 'useful':useful})

  @staticmethod
  def save(actor : MPlayerID, target : MPlayerID, blocked : bool, useful : bool):
    return MEvent(MEventType.SAVE, {'actor':actor, 'target':target, 'blocked':blocked, 'useful':useful})

  @staticmethod
  def milk(actor : MPlayerID, target : MPlayerID, blocked : bool, sniped : bool):
    return MEvent(MEventType.MILK, {'actor':actor, 'target':target, 'blocked':blocked, 'sniped':sniped})

  @staticmethod
  def investigate(actor : MPlayerID, target : MPlayerID, role : str, blocked : bool, sniped : bool):
    return MEvent(MEventType.INVESTIGATE, {'actor':actor, 'target':target, 'role':role, 'blocked':blocked, 'sniped':sniped})

  @staticmethod
  def day():
    return MEvent(MEventType.DAY)

  @staticmethod
  def night():
    return MEvent(MEventType.NIGHT)

  @staticmethod
  def dusk(idiot : MPlayerID, venges : List[MPlayerID]):
    return MEvent(MEventType.DUSK, {'idiot':idiot, 'venges':venges})

  @staticmethod
  def start():
    return MEvent(MEventType.START, {})

  @staticmethod
  def town_win():
    return MEvent(MEventType.TOWN_WIN)

  @staticmethod
  def mafia_win():
    return MEvent(MEventType.MAFIA_WIN)

  @staticmethod
  def charge_die(player : MPlayerID, charge : MPlayerID, role : str, aggressor : MPlayerID):
    return MEvent(MEventType.CHARGE_DIE, {'player':player, 'charge':charge, 'role':role, 'aggressor':aggressor})

  @staticmethod
  def contract_result(player : MPlayerID, charge : MPlayerID, role : str, success : bool):
    return MEvent(MEventType.CONTRACT_RESULT, {'player':player, 'charge':charge, 'role':role, 'success':success})