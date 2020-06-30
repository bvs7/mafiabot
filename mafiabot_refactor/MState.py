from enum import Enum, auto
from typing import List, Union, Optional, Dict
from threading import Lock, Thread

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET

class MPhase(Enum):
  INIT = auto()
  DAWN = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()

class EType(Enum):
  VOTE = auto()
  MTARGET = auto()
  TARGET = auto()
  REVEAL = auto()
  TIMER = auto()

  START = auto()
  ELECT = auto()
  KILL = auto()
  VENGEANCE = auto()
  ELIMINATE = auto()
  CHARGE_DIE = auto()
  DUSK = auto()
  REFOCUS = auto()
  TOWN_WIN = auto()
  MAFIA_WIN = auto()
  CONTRACT_RESULT = auto()
  END = auto()
  NIGHT = auto()
  DAWN = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  INVESTIGATE = auto()
  DAY = auto()

"""
VOTE:
  +ELECT
TARGET:
  +DAWN|+VENGEANCE
REVEAL:
TIMER:
  +DAWN|+NIGHT
START:
  +DAY|+NIGHT
ELECT:
  +ELIMINATE|+DUSK
  +NIGHT|
KILL:
  +ELIMINATE
VENGEANCE:
  +ELIMINATE
  +NIGHT
ELIMINATE:
  ++CHARGE_DIE
  +WIN
CHARGE_DIE:
  +REFOCUS
DUSK:
REFOCUS:
WIN:
  ++CONTRACT_RESULT
  +END
CONTRACT_RESULT:
END:
NIGHT:
DAWN:
  ++STRIP
  ++SAVE
  +KILL
  ++MILK
  ++INVESTIGATE
  +DAY
STRIP:
SAVE:
MILK:
INVESTIGATE:
DAY:
"""

class MEvent:
  
  def __init__(self, typ : EType, **kwargs):
    self.type = typ
    for arg,value in kwargs.items():
      setattr(self, arg, value)

  #TODO external event constructors

class EventHandler:

  def unhandled(self, event):
    pass

  def handleEvent(self, event : MEvent):
    handler = getattr(self, event.type.name, default=self.unhandled)
    handler(event)

class NextEventHandler(EventHandler):
  """ Handler for moving the state machine based on state """

  def __init__(self, mstate):
    self.mstate = mstate
    self.push = mstate.pushEvent

  def VOTE(self, event : MEvent):
    assert(event.type == EType.VOTE)
    
    if event.votee == None:
      return

    if ((event.votee == NOTARGET and event.num_voters >= event.no_kill_thresh) or
      (event.votee != NOTARGET and event.num_voters >= event.thresh)):
      self.push(MEvent(EType.ELECT, actor=event.voter, 
        target=event.votee, nokill=event.votee==NOTARGET))
      return

  def TARGET(self, event : MEvent):
    assert(event.type == EType.TARGET)

    players = self.mstate.players

    if self.mstate.phase == MPhase.DUSK:
      if players[self.mstate.vengeance.idiot].target != None:
        self.push(MEvent(EType.VENGEANCE, actor=event.actor, target=event.target))

    if self.mstate.mafia_target == None:
      return
    if any([t.target == None for t in players.values() 
      if t.role in TARGETING_ROLES]):
      return
    self.push(MEvent(EType.DAWN))

  def REVEAL(self, event : MEvent):
    return

  def TIMER(self, event : MEvent):
    if self.mstate.phase == MPhase.DAY or self.mstate.phase == MPhase.DUSK:
      self.push(MEvent(EType.NIGHT))
      return
    if self.mstate.phase == MPhase.NIGHT:
      self.push(MEvent(EType.DAWN))

  def START(self, event : MEvent):
    if self.mstate.phase == MPhase.DAY:
      self.push(MEvent(EType.DAY))
      return
    if self.mstate.phase == MPhase.NIGHT:
      self.push(MEvent(EType.NIGHT))

  def ELECT(self, event : MEvent):
    if (self.mstate.mrules.idiot_vengeance and 
      event.target != NOTARGET and
      self.mstate.players[event.target].role == "IDIOT"):
        self.push(MEvent(EType.DUSK, idiot=event.target))
        return
    event_list = []
    if event.target != NOTARGET:
      event_list.append(MEvent(EType.ELIMINATE, actor=event.actor, target=event.target))
    event_list.append(MEvent(EType.NIGHT))
    self.push(event_list)

  def KILL(self, event : MEvent):
    if not event.saved and event.target != NOTARGET:
      self.push(MEvent(EType.ELIMINATE, actor=event.actor, target=event.target))
  
  def VENGEANCE(self, event : MEvent):
    if event.target != NOTARGET:
      self.push(MEvent(EType.ELIMINATE, actor=event.actor, target=event.target))

  def ELIMINATE(self, event : MEvent):
    event_list = []
    players = self.mstate.players
    contracts = [(p,p.target) for p in players if players[p].role in CONTRACT_ROLES]
    for p,charge in contracts:
      if charge == event.target:
        event_list.append(MEvent(EType.CHARGE_DIE, 
          actor=p, target=charge, aggressor=event.actor))
    num_players = len(players)
    num_mafia = len([p for p in players if players[p].role in MAFIA_ROLES])
    if num_mafia == 0:
      event_list.append(MEvent(EType.TOWN_WIN))
    elif num_mafia >= num_players / 2:
      event_list.append(MEvent(EType.MAFIA_WIN))

    self.push(event_list)

  def CHARGE_DIE(self, event : MEvent):
    role = self.mstate.players[event.actor].role
    if role in ['GUARD','AGENT']:
      self.push(MEvent(EType.REFOCUS, actor=event.actor,
        target=event.target, aggressor=event.aggressor))




class MState():

  def __init__(self):

    self.event_handlers : List[EventHandler] = [NextEventHandler(self)]
    self.event_list : List[MEvent] = []
    self.event_lock : Lock = Lock()

    self.id : int = 0 ##TODO
    self.day : int = 0
    self.phase : MPhase = MPhase.INIT
    self.timer : Optional[int] = None # TODO: add timer

    self.players : Dict[MPlayerID, MPlayer] = {}

    self.mafia_target : Optional[MPlayerID] = None
    self.mafia_targeter : Optional[MPlayerID] = None # TODO: combine into MMafTarget?

    self.active = True

    thread = Thread()

  def pushEvent(self, event : Union[MEvent, List[MEvent]]):
    if type(event) == MEvent:
      event = [event]
    with self.event_lock:
      self.event_list = event + self.event_list

  def popLoop(self):
    while self.active:
      event = None
      with self.event_lock:
        if len(self.event_list) > 0:
          event = event.pop(0)
      if event != None:
        for event_handler in self.event_handlers:
          event_handler.handleEvent(event)

  def vote(self, voter : MPlayerID, votee : Optional[MPlayerID]):
    event = MEvent(EType.VOTE, voter=voter, votee=votee)

    players = self.players
    voter_p = players[voter]

    event.f_votee = voter_p.vote
    voter_p.vote = votee

    event.num_voters = len([v for v in players if players[v].vote == votee])
    event.num_f_voters = len([v for v in players if players[v].vote == event.f_votee])
    event.num_players = len(players)
    event.thresh = int(event.num_players/2) + 1
    event.no_kill_thresh = event.num_players - event.thresh + 1

    self.pushEvent(event)

  def target(self, actor : MPlayerID, target : Optional[MPlayerID]):
    target_event = MEvent(EType.TARGET, actor=actor, target=target, mafia=False)
    self.pushEvent(target_event)

  def mtarget(self, actor : MPlayerID, target : Optional[MPlayerID]):
    target_event = MEvent(EType.TARGET, actor=actor, target=target, mafia=True)
    self.pushEvent(target_event)

  def reveal(self, actor : MPlayerID):
    reveal_event = MEvent(EType.REVEAL, actor=actor)
    self.pushEvent(reveal_event)

  def timer(self):
    timer_event = MEvent(EType.TIMER)
    self.pushEvent(timer_event)

