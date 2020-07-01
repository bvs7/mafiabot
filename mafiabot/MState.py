from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable
from threading import Lock, Thread

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET

# TODO: Put this somewhere better?
ACT_LOOKUP ={
  "MAFIA":"kill",
  "STRIPPER":"strip",
  "DOCTOR":"save",
  "COP":"investigate",
  "MILKY":"milk",
}

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
  WIN = auto()
  CONTRACT_RESULT = auto()
  END = auto()
  NIGHT = auto()
  DAWN = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  INVESTIGATE = auto()
  DAY = auto()

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

class ChatDisplayEventHandler(EventHandler):
  """ Handler for displaying event info """

  def create_resp_lib(self):
    self.resp_lib = {
      "VOTE_RETRACT": "[{voter}] retracted vote for [{former_votee}]",
      "VOTE":       "[{voter}] votes for [{votee}]",
      "MTARGET":    "[{actor}] prepares to kill [{target}]",
      "TARGET":     "You have targeted [{target}]",
      "REVEAL":     "Reveal: [{actor}]",
      "TIMER_DAY":  "Timer: nokill",
      "TIMER_NIGHT":"Timer: some slept through the night",
      "ELECT":      "[{target}] has been elected to be killed",
      "ELECT_NOKILL":"You have elected not to kill anyone",
      "ELECT_IDIOT": "... They were an IDIOT...",
      "KILL":       "[{target}] was killed by the mafia!",
      "KILL_FAIL_QUIET":  "It seems nobody died last night...",
      "VENGEANCE":  "[{actor}] takes [{target}] with them",
      "ELIMINATE" : "[{target}] was {role}",
      "ELIMINATE_ANON":"[{target}] has died",
      "CHARGE_DIE": "[{target}] has died",
      "CHARGE_KILLED": ", at the hands of [{aggressor}]",
      "DEATH":      "[{player}] was {role}",
      "STRIP":      "You were distracted...",
      "SAVE":       "[{target}] was saved after being attacked by the mafia!",
      "MILK":       "[{target}] received milk in the night.",
      "NO_MILK_SELF": "Ewww, please don't milk yourself in front of me",
      "INVESTIGATE":"[{target}] is {role}",
      "DAWN":"Day dawns",
      "DAY":        "Pick someone to elect.",
      "NIGHT":      "Night falls",
      "NIGHT_OPTIONS":"Pick someone to {act}:\n",
      "DUSK":       "The sky darkens as their reddening eyes observe the crowd...",
      "DUSK_OPTIONS": "Pick someone who voted for you to kill:\n",
      "IDIOT_KILL": "[{actor}] kills [{target}] before the crowd can subdue them",
      "START":      "Start Game:",
      "WIN":   "[{winning_team}] Wins!",
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

  def __init__(self, mstate, cast_main, cast_mafia, send_dm, ids = {}):
    self.mstate = mstate
    self.cast_main = cast_main
    self.cast_mafia= cast_mafia
    self.send_dm   = send_dm
    self.create_resp_lib()
    self.ids = ids # id to name dict

  def comm_format(self, msg, event, notarget="None"):
    msg = msg.format(**(event.__dict__))
    for i,name in self.ids.items():
      msg = msg.replace("[{}]".format(i), name)
    msg.replace("[{}]".format(NOTARGET), notarget)
    return msg

  @staticmethod
  def dispVoteThresh(event, former = False):
    former_str = "former_" if former else ""
    votee = former_str + "votee"
    if event.votee == NOTARGET:
      thresh = "{no_kill_thresh}"
      goal = 'for peace'
    else:
      thresh = "{thresh}"
      goal = 'to elect [{votee}]'.format(votee="{"+votee+"}")
    return "{votes}/{thresh} ".format(votes="{"+former_str+"votes}", thresh=thresh) + goal

  @staticmethod
  def listMenu(players):
    ps = []
    c = 'A'
    for player in players:
      ps.append("{}: [{}]".format(c,player))
      c = chr(ord(c)+1)
    ps.append("{}: [NOTARGET]".format(c))
    return ps

  @staticmethod
  def teamFromRole(role):
    if role in TOWN_ROLES:
      return "Town"
    if role in MAFIA_ROLES:
      return "Mafia"
    if role in ROGUE_ROLES:
      return "Rogue"

  @staticmethod
  def dispRole(role, level="ON"):
    if level in ["ON","ROLE"]:
      return role
    elif level == "TEAM":
      m = ChatDisplayEventHandler.teamFromRole(role)
      return m + " Aligned"
    elif level == "MAFIA":
      m = "Mafia" if ChatDisplayEventHandler.teamFromRole(role)=="Mafia" else "Not Mafia"
      return m + " Aligned"
    else:
      return "[REDACTED]"

  def VOTE(self, event : MEvent):
    # TODO: double check when event info is made?
    msg = self.resp_lib["VOTE"]
    if event.votee == None:
      msg = self.resp_lib["VOTE_RETRACT"]
    else:
      msg += ", {}".format(self.dispVoteThresh(event, former = False))
    if event['former_votee'] != None:
      msg +=", ({})".format(self.dispVoteThresh(event, former = True))
    self.cast_main(self.comm_format(msg, event, notarget="NOKILL"))

  def TARGET(self, event : MEvent):
    if event.mafia:
      msg = self.resp_lib["MTARGET"]
    else:
      msg = self.resp_lib["TARGET"]
      dest = event.actor

    if event.target == NOTARGET:
      msg = "You have decided not to act tonight"
    
    if event.mafia:
      self.cast_mafia(self.comm_format(msg, event))
    else:
      self.send_dm(self.comm_format(msg, event), dest)

  def REVEAL(self, event : MEvent):
    celeb_id = event.actor
    celeb = self.mstate.players[celeb_id]
    if self.mstate.celeb_stripped(celeb_id):
      msg = self.resp_lib["STRIP"]
      self.send_dm(msg, event.actor)
    else:
      msg = self.resp_lib["REVEAL"]
      self.cast_main(self.comm_format(msg, event))

  def TIMER(self, event : MEvent):
    if self.mstate.phase == MPhase.DAY:
      msg = self.resp_lib["TIMER_DAY"]
    elif self.mstate.phase == MPhase.NIGHT:
      msg = self.resp_lib["TIMER_NIGHT"]
    self.cast_main(msg)

  def START(self, event : MEvent):
    msg = "TODO: Start message"
    self.cast_main(msg)

  def ELECT(self, event : MEvent):
    msg = self.resp_lib["ELECT"]
    if self.mstate.players[event.target].role == "IDIOT":
      msg += self.resp_lib["ELECT_IDIOT"]

    self.cast_main(self.comm_format(msg, event, "Nobody"))

  def KILL(self, event : MEvent):
    if event.success:
      msg = self.resp_lib["KILL"]
      self.cast_main(self.comm_format(msg, event, "Nobody"))

  def VENGEANCE(self, event : MEvent):
    msg = self.resp_lib["VENGEANCE"]
    self.cast_main(self.comm_format(msg, event))

  def ELIMINATE(self, event : MEvent):
    msg = self.resp_lib["ELIMINATE"]
    event.role = self.dispRole(self.mstate.players[event.target].role)
    self.cast_main(self.comm_format(msg, event))

  def CHARGE_DIE(self, event : MEvent):
    msg = self.resp_lib["CHARGE_DIE"]
    msg += self.resp_lib["CHARGE_KILLED"]

    self.send_dm(self.comm_format(msg, event), event.actor)

  def DUSK(self, event : MEvent):
    msg = self.resp_lib["DUSK"]
    self.cast_main(self.comm_format(msg, event))
    options = self.resp_lib["DUSK_OPTIONS"]
    options += self.listMenu(self.mstate.vengeance.venges)
    self.send_dm(options, event.actor)

  def REFOCUS(self, event : MEvent):
    if event.actor == event.aggressor:
      msg = self.resp_lib["REFOCUS_SELF"]
      if event.role == "GUARD":
        event.new_role = "IDIOT"
      elif event.role == "AGENT":
        event.new_role = "SURVIVOR"
    else:
      msg = self.resp_lib["REFOCUS"]
      if event.role == "GUARD":
        event.new_role = "AGENT"
      elif event.role == "AGENT":
        event.new_role = "GUARD"
    
    self.send_dm(self.comm_format(msg, event), event.actor)

  def WIN(self, event : MEvent):
    msg = self.resp_lib["WIN"]
    self.cast_main(self.comm_format(msg, event))

  def CONTRACT_RESULT(self, event : MEvent):
    msg = self.resp_lib["CONTRACT_RESULT"]
    event.result = "Won" if event.success else "Lost"
    self.cast_main(self.comm_format(msg, event))

  def END(self, event : MEvent):
    # TODO: reveals?
    pass

  def NIGHT(self, event : MEvent):
    msg = self.resp_lib["NIGHT"]
    self.cast_main(msg)
    targeting_players = [p for p in self.mstate.players.values() if p.role in TARGETING_ROLES]
    for player in targeting_players:
      msg = self.resp_lib["NIGHT_OPTIONS"].format(act = ACT_LOOKUP[player.role])
      msg += self.listMenu(self.mstate.players)
      self.send_dm(self.comm_format(msg, event))

  def DAWN(self, event : MEvent):
    msg = self.resp_lib["DAWN"]
    self.cast_main(msg)

  def STRIP(self, event : MEvent):
    pass

  def SAVE(self, event : MEvent):
    if event.stripped:
      msg = self.resp_lib["STRIP"]
      self.send_dm(msg, event.actor)
    # TODO doc_self stuff?

  def MILK(self, event : MEvent):
    if event.stripped:
      msg = self.resp_lib["STRIP"]
      self.send_dm(msg, event.actor)
    elif event.success and event.target in self.mstate.players:
      msg = self.resp_lib["MILK"]
      self.cast_main(self.comm_format(msg, event))

  def INVESTIGATE(self, event : MEvent):
    if event.stripped:
      msg = self.resp_lib["STRIP"]
      self.send_dm(msg, event.actor)
    elif event.success and event.target in self.mstate.players:
      msg = self.resp_lib["INVESTIGATE"]
      event.role = self.mstate.players[event.target].role
      self.send_dm(self.comm_format(msg, event), event.actor)

  def DAY(self, event : MEvent):
    msg = self.resp_lib["DAY"]
    self.cast_main(msg)

class UpdateStateEventHandler(EventHandler):
  """ Handler for updating the game state """

  def __init__(self, mstate):
    self.mstate = mstate

  def VOTE(self, event : MEvent):
    
    players = self.mstate.players
    voter = players[event.voter]
    # assertions?
    if event.votee in players or event.votee in (NOTARGET, None):
      voter.vote = event.votee

    event.num_voters = len([v for v in players if players[v].vote == event.votee])
    event.num_f_voters = len([v for v in players if players[v].vote == event.former_votee])
    event.num_players = len(players)
    event.thresh = int(event.num_players/2) + 1
    event.no_kill_thresh = event.num_players - event.thresh + 1

  def TARGET(self, event : MEvent):
    # TODO: assertions?

      if event.mafia:
        self.mstate.mafia_target = event.target
        self.mstate.mafia_targeter = event.actor
      else:
        actor = self.mstate.players[event.actor]
        actor.target = event.target

  def REVEAL(self, event : MEvent):
    pass

  def TIMER(self, event : MEvent):
    pass

  def START(self, event : MEvent):
    # TODO: start!
    pass

  def ELECT(self, event : MEvent):
    player = self.mstate.players[event.target]
    if player.role == "IDIOT":
      contract = self.mstate.contracts[event.target]
      contract['success'] = True

  def KILL(self, event : MEvent):
    self.mstate.mafia_target = None
    self.mstate.mafia_targeter = None

  def VENGEANCE(self, event : MEvent):
    pass

  def ELIMINATE(self, event : MEvent):
    if event.target in self.mstate.players:
      del self.mstate.players[event.target]

  def CHARGE_DIE(self, event : MEvent):
    contract = self.mstate.contracts[event.actor]
    if contract['role'] == "AGENT":
      contract['success'] = True
    elif contract['role'] in ["GUARD", "SURVIVOR"]:
      contract['success'] = False

  def DUSK(self, event : MEvent):
    pass

  def REFOCUS(self, event : MEvent):
    actor = event.actor
    charge = event.target
    aggressor = event.aggressor
    role = event.role

    needed_alive = role in ['GUARD', 'SURVIVOR']

    if charge == aggressor or actor == aggressor or not aggressor in self.players:
      new_charge = actor
      new_role = "IDIOT" if needed_alive else "SURVIVOR"
    else:
      new_charge = aggressor
      new_role = "AGENT" if needed_alive else "GUARD"
    player = self.mstate.players[actor]
    player.role = new_role
    player.target = new_charge
    # Update contract!
    contract = self.mstate.contracts[event.actor]
    contract['role'] = player.role
    contract['target'] = player.target
    contract['success'] = player.role in ["SURVIVOR", "GUARD"]
  

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
        self.push(MEvent(EType.DUSK, idiot=event.target, 
          actor=event.actor, target=event.target))
        return
    event_list = []
    if event.target != NOTARGET:
      event_list.append(MEvent(EType.ELIMINATE, actor=event.actor, 
        target=event.target))
    event_list.append(MEvent(EType.NIGHT))
    self.push(event_list)

  def KILL(self, event : MEvent):
    if not event.saved and event.target != NOTARGET:
      self.push(MEvent(EType.ELIMINATE, actor=event.actor, target=event.target))
  
  def VENGEANCE(self, event : MEvent):
    event_list = []
    if event.target != NOTARGET and event.target != None:
      event_list.append(MEvent(EType.ELIMINATE, actor=event.actor, target=event.target))
    event_list.append(MEvent(EType.ELIMINATE, actor=self.mstate.vengeance.final_vote,
      target=self.mstate.vengeance.idiot))
    self.push(event_list)

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
      event_list.append(MEvent(EType.WIN, winning_team="Town"))
    elif num_mafia >= num_players / 2:
      event_list.append(MEvent(EType.WIN, winning_team="Mafia"))

    self.push(event_list)

  def CHARGE_DIE(self, event : MEvent):
    role = self.mstate.players[event.actor].role
    if role in ['GUARD','AGENT']:
      self.push(MEvent(EType.REFOCUS, actor=event.actor,
        target=event.target, aggressor=event.aggressor, role=role))

  def DUSK(self, event : MEvent):
    pass

  def REFOCUS(self, event : MEvent):
    pass

  def WIN(self, event : MEvent):
    # Check the state of contracts, decide winners/losers?
    event_list = []
    for contract in self.mstate.contracts:
      event_list.append(MEvent(EType.CONTRACT_RESULT, **contract))
    event_list.append(MEvent(EType.END, winning_team=event.winning_team))

    self.push(event_list)

  def CONTRACT_RESULT(self, event : MEvent):
    pass

  def END(self, event : MEvent):
    # TODO: implement end state stuff?
    pass

  def NIGHT(self, event : MEvent):
    pass

  def DAWN(self, event : MEvent):
    event_list = []
    for strip in event.strips:
      event_list.append(MEvent(EType.STRIP, **strip))
    for save in event.saves:
      event_list.append(MEvent(EType.SAVE, **save))
    if not event.kill == None:
      event_list.append(MEvent(EType.KILL, **event.kill))
    for milk in event.milks:
      event_list.append(MEvent(EType.MILK, **milk))
    for investigate in event.investigates:
      event_list.append(MEvent(EType.INVESTIGATE, **investigate))
    event_list.append(MEvent(EType.DAY))

    self.push(event_list)

  def STRIP(self, event : MEvent):
    pass

  def SAVE(self, event : MEvent):
    pass

  def MILK(self, event : MEvent):
    pass

  def INVESTIGATE(self, event : MEvent):
    pass

  def DAY(self, event : MEvent):
    pass


class MState():

  def __init__(self, 
      cast_main : Callable[[str],None],
      cast_mafia : Callable[[str],None],
      send_dm : Callable[[str,MPlayerID],None],
      ids : Dict[MPlayerID, str]
    ):

    self.event_handlers : List[EventHandler] = []
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

    self.contracts = {} #Dicts? player_id -> role, target, success

    self.vengeance = None # venges, final_vote, idiot.

    self.active = True

    self.initEventHanders(cast_main, cast_mafia, send_dm, ids)

    thread = Thread(target=self.popLoop, name="MState thread")

    thread.start()

  def initEventHanders(self,
      cast_main : Callable[[str],None],
      cast_mafia : Callable[[str],None],
      send_dm : Callable[[str,MPlayerID],None],
      ids : Dict[MPlayerID, str]
    ):
    self.event_handlers = [
      ChatDisplayEventHandler(self, cast_main, cast_mafia, send_dm),
      UpdateStateEventHandler(self),
      NextEventHandler(self)
    ]

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

  
  ### Status checking functions:

  def celeb_stripped(self, celeb_id):
    return celeb_id in self.stripped
