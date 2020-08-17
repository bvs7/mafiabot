from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET

# TODO: ELECT/DAWN events remove TIMER from queue (and stop timer later)

class MPhase(Enum):
  INIT = auto()
  DAWN = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()

class EndGameException(Exception):
  pass

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
    for id,role in zip(self.ids, self.roles):
      send_dm(ROLE_EXPLAIN[role], id)
      if role in CONTRACT_ROLES:
        (role,charge,succes) = self.contracts[id]
        send_dm(default_resp_lib["CHARGE_ASSIGN"].format(charge=charge),id)
    roleDict = makeRoleDict(self.roles)
    msg = default_resp_lib["START"]
    msg += dispKnownRoles(roleDict, self.known_roles)
    cast_main(msg)

  def write(self, mstate):
    for i,role in zip(self.ids, self.roles):
      player = MPlayer(i,role)
      mstate.players[i] = player
    mstate.player_order = list(mstate.players.keys())
    self.phase = MPhase.DAY
    mstate.day = 0
    if (self.start_night == "ON" or 
      (self.start_night == "ODD" and len(self.ids) % 2 == 1) or
      (self.start_night == "EVEN" and len(self.ids) % 2 == 0)):
      self.phase = MPhase.NIGHT
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
    self.players = mstate.player_order
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
        reveal = dispRole(reveal, self.cop_strength)
        msg = default_resp_lib["INVESTIGATE"].format(target=self.target,role=reveal)
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
        mstate.vengeance = {'venges':self.venges,
          'final_vote':self.actor, 'idiot':self.target}
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
    event_list.append(ELIMINATE(self.vengeance['final_vote'], self.vengeance['idiot']))
    event_list.append(NIGHT())
    push(event_list)

class ELIMINATE(MEvent):
  def __init__(self, actor, target):
    self.actor = actor
    self.target = target

  def read(self, mstate):
    self.role = mstate.players[self.target].role
    self.reveal_on_death = mstate.rules[reveal_on_death]
    self.start_roles = mstate.start_roles

  def msg(self, cast_main, cast_mafia, send_dm):
    reveal = dispRole(self.role, self.reveal_on_death)
    msg = default_resp_lib["ELIMINATE"].format(target=self.target,role=reveal)
    cast_main(msg)
    msg = default_resp_lib["SHOW_ROLES"].format(dispStartRoles(self.start_roles))
    send_dm(msg, self.target)

  def write(self, mstate):
    del mstate.players[self.target]
    mstate.player_order = list(mstate.players.keys())
    
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
      result="Won!" if self.success else "Lost!",
      charge=self.charge
    )
    cast_main(msg)

class END(MEvent):
  def __init__(self, winning_team):
    self.winning_team = winning_team

  def next(self, push):
    # TODO: implement end state stuff
    raise EndGameException("{} Won!".format(self.winning_team))
