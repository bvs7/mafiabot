from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable, Any
from threading import Lock, Thread
import json

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET
from .MEvent import MEvent, START, VOTE, TARGET, MTARGET, REVEAL, TIMER, END, MPhase, EndGameException
from .MRules import MRules
from .MVengeance import MVengeance

# Static rolegen

class MState:

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

    # General Game State
    self.id : int = 0 ## TODO
    self.day : int = 0
    self.phase : MPhase = MPhase.INIT

    self.players : Dict[MPlayerID, MPlayer] = {}
    self.player_order = []
    self.contracts = {} #Dicts? player_id -> (role, target, success)

    self.mafia_target : Optional[MPlayerID] = None
    self.mafia_targeter : Optional[MPlayerID] = None
    self.stripped = []
    self.stunned = []
    self.revealed = []
    self.vengeance : Optional[MVengeance] = None 

    self.timer_inst : Optional[int] = None # TODO: add timer

    # Event_List
    self.event_list : List[MEvent] = []
    self.event_lock : Lock = Lock()
    self.active = True
    self.thread = Thread(target=self.popLoop, name="MState thread")
    self.thread.start()

    self.start_roles = {}

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
        self.end_callback(e)

  def handleEvent(self, event):
    event.read(self)
    event.msg(self.cast_main, self.cast_mafia, self.send_dm)
    event.write(self)
    event.next(self.pushEvent)

  def start(self, ids : List[MPlayerID], roles : List[str], contracts):
    self.start_roles = dict(zip(ids,roles))
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
    for player in self.players:
      voteDict[player] = 0
    for player in self.players.values():
      if not player.vote == None:
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

  def to_json(self):
    d = {
      "id":self.id,
      "day":self.day,
      "phase":self.phase.name,
      "players":[p.to_json() for p in self.players.values()],
      "contracts":self.contracts,
      "start_roles":self.start_roles,
      "rules":self.rules.rules
    }
    if not self.mafia_target == None:
      d["mafia_target"] = self.mafia_target
      d["mafia_targeter"] = self.mafia_targeter
    if not len(self.stripped) == 0:
      d['stripped'] = self.stripped
    if not len(self.stunned) == 0:
      d['stunned'] = self.stunned
    if not len(self.revealed) == 0:
      d['reveal'] = self.reveal
    if not self.vengeance == None:
      d['vengeance'] = self.vengeance.to_json()
    return d

  @staticmethod
  def from_json(d, main_cast, mafia_cast, send_dm, end_callback):
    r = MRules()
    r.rules = d["rules"]
    s = MState(main_cast, mafia_cast, send_dm, r, end_callback)
    s.id = d["id"]
    s.day = d["day"]
    phase = d["phase"]
    if phase == "DAY":
      s.phase = MPhase.DAY
    elif phase == "NIGHT":
      s.phase = MPhase.NIGHT
    elif phase == "DUSK":
      s.phase = MPhase.DUSK
    s.players = {}
    for player in d["players"]:
      vote = None if not "vote" in player else player['vote']
      target = None if not "target" in player else player['target']
      id = player["id"]
      role = player["role"]
      s.players[id] = MPlayer(id, role, vote, target)
    s.player_order = list(s.players.keys())
    s.contracts = d["contracts"]
    s.start_roles = d["start_roles"]
    if "mafia_target" in d:
      s.mafia_target = d["mafia_target"]
      s.mafia_targeter = d["mafia_targeter"]
    if "stripped" in d:
      s.stripped = d["stripped"]
    if "stunned" in d:
      s.stunned = d["stunned"]
    if "revealed" in d:
      s.revealed = d["revealed"]
    if "vengeance" in d:
      s.vengeance = MVengeance.from_json(d["vengeance"])
    return s

