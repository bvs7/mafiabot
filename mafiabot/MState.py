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
      rules : MRules,
      end_callback,
    ):
    self.cast_main = cast_main
    self.cast_mafia= cast_mafia
    self.send_dm   = send_dm

    self.rules = rules

    self.end_callback = end_callback

    self.event_list : List[MEvent] = []
    self.event_lock : Lock = Lock()

    self.id : int = 0 ## TODO
    self.day : int = 0
    self.phase : MPhase = MPhase.INIT
    self.timer_inst : Optional[int] = None # TODO: add timer

    self.players : Dict[MPlayerID, MPlayer] = {}
    self.player_order = []
    self.start_roles = []

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
        self.end_callback(e)

  def handleEvent(self, event):
    event.read(self)
    event.msg(self.cast_main, self.cast_mafia, self.send_dm)
    event.write(self)
    event.next(self.pushEvent)

  def start(self, ids, roles, contracts={}):
    self.start_roles = roles
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

  def phaseDateStatus(self):
    return "Game {}: {} {}\n".format(self.id, self.phase.name, self.day)

  def timerStatus(self):
    if self.timer_inst != None:
      return "{}\n".format(self.timer_inst.getTime())
    else:
      return ""

  def voteStatus(self):
    msg = ""
    voteDict = {}
    for player in self.players.values():
      if not player.vote == None:
        if not player.vote in voteDict:
          voteDict[player.vote] = 0
        voteDict[player.vote] += 1
    num_players = len(self.players)
    thresh = int(num_players/2)+1
    no_kill_thresh = num_players - thresh + 1
    for player_id in self.player_order:
      if player_id in voteDict:
        msg += "  [{}]: {}/{}\n".format(player_id, voteDict[player_id], thresh)
    if "NOTARGET" in voteDict:
      msg += "  [{}]: {}/{}\n".format("NOTARGET", voteDict["NOTARGET"], no_kill_thresh)
    return msg

  def roleStatus(self):
    know_role = self.rules[known_roles]
    rev_death = self.rules[reveal_on_death]
    roleDict = makeRoleDict([p.role for p in self.players.values()])
    msg = ""

    if know_role == "OFF" or rev_death == "OFF":
      pass
    elif know_role == "MAFIA" or rev_death == "MAFIA":
      msg += dispKnownRoles(roleDict, "MAFIA")
    elif know_role == "TEAM" or know_role == "TEAM":
      msg += dispKnownRoles(roleDict, "TEAM")
    elif know_role == "ROLE" and rev_death == "ROLE":
      msg += dispKnownRoles(roleDict, "ROLE")
    return msg

  def nightOptions(self):
    msg = ""
    msg = default_resp_lib["NIGHT_OPTIONS"]
    msg += "\n".join(listMenu(self.player_order)) + "\n"
    return msg

  # General status of mstate
  def main_status(self):
    msg = ""
    msg += self.phaseDateStatus()
    msg += self.timerStatus()
    if self.phase == MPhase.DAY:
      msg += self.voteStatus()

    msg += self.roleStatus()
    return msg

  def mafia_status(self):
    msg = ""
    msg += self.phaseDateStatus()
    msg += self.timerStatus()
    if self.phase == MPhase.DAY:
      msg += self.voteStatus()
    elif self.phase == MPhase.NIGHT:
      msg += "Current target: [{}]\n".format(self.mafia_target)
      msg += self.nightOptions()
    msg += self.roleStatus()
    return msg

  def dm_status(self, player_id):
    msg = ""
    msg += self.phaseDateStatus()
    msg += self.timerStatus()
    if self.phase == MPhase.DAY:
      if player_id in self.players:
        msg += "Current vote: [{}]\n".format(self.players[player_id].vote)
      msg += self.voteStatus()
    elif self.phase == MPhase.NIGHT:
      if player_id in self.players and self.players[player_id].role in TARGETING_ROLES:
        msg += "Current target: [{}]\n".format(self.players[player_id].target)
      msg += self.nightOptions()
    msg += self.roleStatus()
    return msg