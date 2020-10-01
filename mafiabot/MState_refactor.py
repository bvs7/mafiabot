from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable, Any, Tuple
from threading import Lock, Thread
import json
from collections import OrderedDict

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, MRole, NOTARGET
from .MEvent import MEvent, START, VOTE, TARGET, MTARGET, REVEAL, TIMER, END
from .MRules import MRules

class MPhase(Enum):
  INIT = auto()
  DAWN = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()

class EndGameException(Exception):
  pass

class MContract:
  def __init__(self, role:MRole, charge:MPlayerID, success:bool):
    self.role=role
    self.charge=charge
    self.success=success

  def to_json(self):
    return self.__dict__

  @staticmethod
  def from_json(d):
    return MContract(d['role'],d['target'],d['success']=="True")


class MVengeance:
  def __init__(self, venges : List[MPlayerID], final_vote : MPlayerID, idiot : MPlayerID):
    self.venges = venges
    self.final_vote = final_vote
    self.idiot = idiot

  def to_json(self):
    d = {
      'venges':self.venges,
      'final_vote':self.final_vote,
      'idiot':self.idiot,
    }
    return d

  @staticmethod
  def from_json(d):
    return MVengeance(d['venges'], d['final_vote'], d['idiot'])

class MState_refactor:

  def __init__(self, id:int, rules:MRules, cast_main=print, cast_mafia=print, send_dm=print):

    self.cast_main = cast_main
    self.cast_mafia = cast_mafia
    self.send_dm = send_dm

    self.rules = rules

    self.id = id
    self.day = 0
    self.phase = MPhase.INIT

    self.players : Dict[MPlayerID, MPlayer] = OrderedDict() # maps players to playerids
    self.contracts : Dict[MPlayerID, MContract] = {} #Dicts? player_id -> (role, target, success)

    self.mafia_target : Optional[Tuple[MPlayerID, MPlayerID]] = None
    self.stripped : List[MPlayerID] = [] # Keep track of who is stripped through dawn calc and during next day (celeb).
    self.stunned : List[MPlayerID] = [] # Keep track of who is stunned (GOON, or by IDIOT) at night.
    self.revealed : List[MPlayerID] = [] # Keep track of which CELEBs have revealed.
    self.vengeance : Optional[MVengeance] = None # Used when an IDIOT needs to get revenge.

    self.start_roles = "Init"

  def vote(self, voter_id : MPlayerID, votee_id : Optional[MPlayerID]):
    # TODO: add validation stub?
    # Check day time
    # Check voter is alive
    # Check votee is alive


    return self.__vote(voter_id, votee_id)

  def __vote(self, voter_id : MPlayerID, votee_id : Optional[MPlayerID]):
    players = self.players
    voter = players[voter_id]
    f_votee_id = voter.vote

    # Change vote
    if voter_id in players or voter_id in (NOTARGET, None):
      voter.vote = votee_id

    # Calculate thresholds
    n_voters = len([v for v in players if players[v].vote == votee_id])
    n = len(players)
    thresh = int(n/2) + 1
    nokill_thresh = n - thresh + 1

    # Send update
    msg = ""
    if f_votee_id == None:
      msg += resp_lib['VOTE'].format(voter=voter_id, votee=votee_id)
    else:
      if votee_id == None:
        msg += resp_lib['VOTE_RETRACT'].format(voter=voter_id, f_votee=f_votee_id)
      else:
        msg += resp_lib['VOTE_CHANGE'].format(voter=voter_id, votee=votee_id, f_votee=f_votee_id)

    if votee_id in players:
      msg += resp_lib['VOTE_UPDATE'].format(votee=votee_id, n_voters=n_voters, thresh=thresh)
    elif votee_id == NOTARGET:
      msg += resp_lib['VOTE_UPDATE_NOKILL'].format(n_voters=n_voters, nokill_thresh=nokill_thresh)

    self.cast_main(msg)

    # Check for end of DAY
    if votee_id == None:
      return

    over_nokill_thresh = votee_id == NOTARGET and n_voters >= nokill_thresh
    over_thresh = votee_id != NOTARGET and n_voters >= thresh

    if over_nokill_thresh or over_thresh:
      self.__elect(voter_id, votee_id)
      return
  
  def __elect(self, actor_id, target_id):
    msg = ""
    if not target_id == NOTARGET:
      target = self.players[target_id]

      msg += resp_lib['ELECT'].format(target=target_id)

      if target.role == "IDIOT":
        self.contracts[target_id].success = True
        self.vengeance = MVengeance([p_id for p,p_id in self.players.items() if p.vote == target_id], actor_id, target_id)

        idiot_vengeance = self.rules[idiot_vengeance]
        if not idiot_vengeance == "OFF":
          msg += resp_lib["ELECT_IDIOT"]

          if idiot_vengeance == "DAY":
            msg += resp_lib["ELECT_DAY"]
            self.cast_main(msg)
            return self.__day()

          elif idiot_vengeance == "STUN":
            msg += resp_lib["ELECT_STUN"]
            self.stunned = self.vengeance.venges
            for p_id in self.stunned:
              self.send_dm(resp_lib["STUN"], p_id)

          elif idiot_vengeance == "KILL":
            return self.__dusk(target_id) # Go to dusk, don't kill idiot yet
          elif idiot_vengeance == "WIN":
            return self.__win("IDIOT", target_id)

      self.__eliminate(actor_id, target_id) 
    else:
      msg += resp_lib['ELECT_NOKILL']

    self.cast_main(msg)
    self.__night()

  def __eliminate(self, actor_id, target_id):
    role = self.players[target_id].role
    reveal = dispRole(role, self.rules[reveal_on_death])

    msg = resp_lib["ELIMINATE"].format(target=target_id, role=reveal)
    self.cast_main(msg)

    del self.players[target_id]

    for p,contract in self.contracts.items():
      if contract.charge == target_id:
        # charge has died
        if contract.role == "AGENT":
          contract.success = True
        elif contract.role in ("GUARD", "SURVIVOR"):
          contract.success = False
        if ((contract.role == "GUARD" and self.rules[charge_refocus_guard]) or
          (contract.role == "AGENT" and self.rules[charge_refocus_agent])):
          self.__refocus(p, target_id, actor_id, contract.role)

    n_players = len(self.players)
    n_mafia = len([p for p in self.players.values() if p.role in MAFIA_ROLES])
    if n_mafia == 0:
      self.__win("Town")
      return
    elif n_mafia>= (n_players+1) // 2:
      self.__win("Mafia")
      return

    msg = resp_lib["SHOW_ROLES"].format(dispStartRoles(self.start_roles))
    self.send_dm(msg, target_id)

  def __night(self):
    msg = resp_lib['NIGHT']
    self.cast_main(msg)

    opts = resp_lib['NIGHT_OPTIONS']
    opts += '\n'.join(listMenu(self.players))
    for t_p in [p for p in self.players if self.players[p].role in TARGETING_ROLES]:
      msg = opts
      if t_p in self.stunned:
        msg = resp_lib["STUNNED"] + "\n" + opts
      self.send_dm(msg, t_p)
    self.cast_mafia(opts)

    self.phase = MPhase.NIGHT
    for player in self.players.values():
      player.vote = None
    self.stripped = []
    self.vengeance = None

  def target(self, actor_id : MPlayerID, target_id : Optional[MPlayerID]):
    # TODO: validation
    return self.__target(actor_id, target_id)

  def __target(self, actor_id : MPlayerID, target_id : Optional[MPlayerID]):
    actor = self.players[actor_id]
    actor.target = target_id

    if actor.target == NOTARGET:
      msg = resp_lib["NOTARGET"]
    else:
      msg = resp_lib["TARGET"].format(target=actor.target)
    self.send_dm(msg, actor.id)

    if (self.mafia_target != None) and all([p.target != None for p in self.players.values() if p.role in TARGETING_ROLES]):
      self.__dawn()
    return

  def mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    # TODO: validation
    return self.__mtarget(targeter_id, target_id)

  def __mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.mafia_target = (target_id, targeter_id)
    msg = resp_lib["MTARGET"].format(actor=targeter_id, target=target_id)
    self.cast_mafia(msg)
    if (self.mafia_target != None) and all([p.target != None for p in self.players.values() if p.role in TARGETING_ROLES]):
      self.__dawn()
    return

  def __dawn(self):
    msg = resp_lib['DAWN'] + "\n"
    players = self.players
    
    for stripper_id in [p for p in players if players[p].role == "STRIPPER"]:
      stripper = players[stripper_id]
      if not stripper.target in (NOTARGET, None):
        self.stripped.append(stripper.target)
        _know_if_stripped = self.rules[know_if_stripped]
        target = players[stripper.target]

        msg = resp_lib["STRIP"]
        if _know_if_stripped == "ON":
          self.send_dm(msg, target.id)
        elif _know_if_stripped == "TARGET":
          if target.role in TARGETING_ROLES or target.role == "CELEB":
            self.send_dm(msg, target.id)

    target_saved = False
    target_savior = None
    if not self.mafia_target[0] in (NOTARGET, None):
      for doctor_id in [p for p in players if players[p].role == "DOCTOR"]:
        doctor = players[doctor_id]
        if not doctor.target in (NOTARGET, None):
          success = doctor.target == self.mafia_target[0]
          stripped = doctor_id in self.stripped

          if self.rules[know_if_stripped] == "USEFUL" and success and stripped:
            self.send_dm(resp_lib["STRIP"], doctor_id)

          if success and not stripped:
            target_saved = True
            if know_if_saved_doc == "ON":
              self.send_dm(resp_lib["SAVE_DOC"], doctor_id)
    
    if self.mafia_target[0] in (NOTARGET, None):
      msg += resp_lib["KILL_FAIL_QUIET"]
    else:
      if target_saved:
        if self.rules[know_if_saved] == "OFF":
          msg += resp_lib["KILL_FAIL_QUIET"]
        elif self.rules[know_if_saved] == "SECRET":
          msg += resp_lib["SAVE_SECRET"]
        elif self.rules[know_if_saved] == "SAVED":
          msg += resp_lib["SAVE"].format(target=self.mafia_target[0])
      else:
        msg += resp_lib["KILL"].format(target=self.mafia_target[0])
        self.__eliminate(self.mafia_target[1], self.mafia_target[0])
    
    for milky_id in [p for p in players if players[p].role == "MILKY"]:
      milky = players[milky_id]
      if not milky.target in (NOTARGET, None):
        stripped = milky_id in self.stripped
        success = milky.target in players

        if self.rules[know_if_stripped] == "USEFUL" and success and stripped:
          self.send_dm(resp_lib["STRIP"], milky_id)

        if not stripped and success:
          msg += resp_lib["MILK"].format(target=milky.target)

    for cop_id in [p for p in players if players[p].role == "COP"]:
      cop = players[cop_id]
      if not cop.target in (NOTARGET, None):
        stripped = cop_id in self.stripped
        success = cop.target in players

        if self.rules[know_if_stripped] == "USEFUL" and success and stripped:
          self.send_dm(resp_lib["STRIP"], cop_id)
        
        if not stripped and success:
          role = players[cop.target].role
          if role == "GODFATHER":
            role = "TOWN"
          elif role == "MILLER":
            role = "MAFIA"
          reveal = dispRole(role, self.rules[cop_strength])
          self.send_dm(resp_lib["INVESTIGATE"].format(target=cop.target, role=reveal))

    self.day += 1

    self.cast_main(msg)

    return self.__day()

  def __day(self):
    self.phase = MPhase.DAY
    self.mafia_target = None
    for p in self.players.values():
      p.target = None
    self.stunned = []

  def __dusk(self, idiot_id):
    self.phase = MPhase.DUSK
    self.cast_main(resp_lib["DUSK"])
    opts = resp_lib["DUSK_OPTIONS"]
    opts += "\n".join(listMenu(self.vengeance.venges, notarget=False))
    self.send_dm(opts, self.vengeance.idiot)

  def itarget(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    # TODO: validation
    return self.__itarget(idiot_id, target_id)
    
  def __itarget(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.__vengeance(idiot_id, target_id)

  def __vengeance(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    msg = resp_lib['VENGEANCE'].format(actor=idiot_id, target=target_id)
    self.cast_main(msg)
    self.__eliminate(idiot_id, target_id)
    self.__eliminate(self.vengeance.final_vote, idiot_id)
    self.__night()

  def __win(self, team, extra=None):
    pass

  def __refocus(self, actor, target, aggressor, role):
    # Make sure there is room for aggressor already being dead (IDIOT vengeance)
    pass
