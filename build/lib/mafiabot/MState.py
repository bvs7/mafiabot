from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable
from threading import Lock, Thread

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET
from .MEvent import MEvent, START, VOTE, TARGET, MTARGET, REVEAL, TIMER, END, MPhase, EndGameException
from .MRules import MRules


class MState():

  def __init__(self, 
      cast_main : Callable[[str],None],
      cast_mafia : Callable[[str],None],
      send_dm : Callable[[str,MPlayerID],None],
      rules,
    ):
    self.cast_main = cast_main
    self.cast_mafia= cast_mafia
    self.send_dm   = send_dm

    self.rules = rules

    self.event_list : List[MEvent] = []
    self.event_lock : Lock = Lock()

    self.id : int = 0 ## TODO
    self.day : int = 0
    self.phase : MPhase = MPhase.INIT
    self.timer_inst : Optional[int] = None # TODO: add timer

    self.players : Dict[MPlayerID, MPlayer] = {}

    self.mafia_target : Optional[MPlayerID] = None
    self.mafia_targeter : Optional[MPlayerID] = None # TODO: combine into MMafTarget?

    self.stripped = []

    self.stunned = []

    self.revealed = []

    self.contracts = {} #Dicts? player_id -> role, target, success

    self.vengeance = None # {venges, final_vote, idiot}

    self.active = True

    self.thread = Thread(target=self.popLoop, name="MState thread")

    self.thread.start()

  def appendEvent(self, event : Union[MEvent, List[MEvent]]):
    if not type(event) == list:
      event = [event]
    with self.event_lock:
      self.event_list = self.event_list + event

  def pushEvent(self, event : Union[MEvent, List[MEvent]]):
    if not type(event) == list:
      event = [event]
    with self.event_lock:
      self.event_list = event + self.event_list

  def popLoop(self):
    while self.active:
      try:
        event = None
        with self.event_lock:
          if len(self.event_list) > 0:
            event = self.event_list.pop(0)
        if event != None:
          self.handleEvent(event)
      except EndGameException as e:
        self.active = False
        print("Caught end of game: {}".format(e))

  def handleEvent(self, event):
    event.read(self)
    event.msg(self.cast_main, self.cast_mafia, self.send_dm)
    event.write(self)
    event.next(self.pushEvent)

  def start(self, ids, roles, contracts={}):
    event = START(ids, roles, contracts)
    self.pushEvent(event)

  def vote(self, voter : MPlayerID, votee : Optional[MPlayerID]):
    event = VOTE(voter, votee)

    self.appendEvent(event)

  def target(self, actor : MPlayerID, target : Optional[MPlayerID]):
    target_event = TARGET(actor, target)
    self.appendEvent(target_event)

  def mtarget(self, actor : MPlayerID, target : Optional[MPlayerID]):
    target_event = MTARGET(actor, target)
    self.appendEvent(target_event)

  def reveal(self, actor : MPlayerID):
    reveal_event = REVEAL(actor)
    self.appendEvent(reveal_event)

  def timer(self):
    timer_event = TIMER()
    self.appendEvent(timer_event)

  def close(self):
    end_event = END("CLOSE")
    self.appendEvent(end_event)
    self.thread.join()
  
  ### Status checking functions:

  def checkStripped(self, p_id):
    return p_id in self.stripped
    
  def checkRevealed(self, p_id):
    return p_id in self.revealed
