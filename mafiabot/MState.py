from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable, Any, Tuple, Set
from threading import Lock, Thread
import json
from collections import OrderedDict

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET
from .MEvent import MEvent, START, VOTE, TARGET, MTARGET, REVEAL, TIMER, END
from .MRules import MRules
from .util import VEnum

__all__ = ['MState','MPhase','MContract','MVengeance','EndGameException','IdiotWinException','TeamWinException']

class MPhase(VEnum):
  INIT = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()

  def to_json(self):
    return self.name
  
  @staticmethod
  def from_json(d):
    return getattr(MPhase, d['__MPhase__'])

class EndGameException(Exception):
  pass

class IdiotWinException(EndGameException):

  def __init__(self, idiot_id, message=None):
    self.idiot_id = idiot_id
    if message == None:
      message = resp_lib["IDIOT_WIN"].format(idiot=idiot_id)
    self.message = message
    super().__init__(self.message)

class TeamWinException(EndGameException):
  def __init__(self, team, message=None):
    self.team = team
    if message == None:
      message = resp_lib["WIN"].format(winning_team=team)
    self.message = message
    super().__init__(self.message)

class MContract:
  def __init__(self, role:MRole, charge:MPlayerID, success:bool):
    self.role=role
    self.charge=charge
    self.success=success

  def to_json(self):
    return {'role':self.role, 'charge':self.charge, 'success':self.success}

  @staticmethod
  def from_json(d):
    return MContract(d['role'],d['charge'],d['success'])


class MVengeance:
  def __init__(self, venges : List[MPlayerID], final_vote : MPlayerID, idiot : MPlayerID):
    self.venges = venges
    self.final_vote = final_vote
    self.idiot = idiot

  def to_json(self):
    return {'venges':self.venges, 'final_vote':self.final_vote, 'idiot':self.idiot}

  @staticmethod
  def from_json(d):
    return MVengeance(d['venges'], d['final_vote'], d['idiot'])

class MState:

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
    self.stunned : Set[MPlayerID] = set() # Keep track of who is stunned (GOON, or by IDIOT) at night.
    self.revealed : List[MPlayerID] = [] # Keep track of which CELEBs have revealed.
    self.vengeance : Optional[MVengeance] = None # Used when an IDIOT needs to get revenge.

    self.start_roles = "Init"

  def start(self, ids, roles, contracts):
    # Check inputs for validity...
    # Ensure starting roles are valid
    # Ensure starting contracts are valid
    rs = [MRole(r) for r in roles]
    self.__start(ids, rs, contracts)

  def __start(self, ids, roles, contracts):
    # Do start things
    self.start_roles = dispStartRoles(ids,roles)
    self.contracts = contracts
    
    for id,role in zip(ids,roles):
      p = MPlayer(id, role)
      self.players[id] = p
    # TODO: start msg
    self.cast_main(resp_lib["START"])
    self.cast_mafia(resp_lib["START_MAFIA"])
    # TODO: check start_night!
    # for now, assume start_night is EVEN
    self.day = 1
    start_night = len(self.players) % 2 == 0
    if start_night:
      self.__night()
    else:
      self.__day()

    return

  def vote(self, voter_id : MPlayerID, votee_id : Optional[MPlayerID]):
    # Check phase
    if not self.phase == MPhase.DAY:
      self.send_dm(resp_lib["INVALID_VOTE_PHASE"], voter_id)
      return
    # Check voter is alive
    if not voter_id in self.players:
      self.send_dm(resp_lib["INVALID_VOTER"].format(player_id=voter_id), voter_id)
      return
    # Check votee is alive
    if not (votee_id in self.players or votee_id in (NOTARGET, None)):
      self.send_dm(resp_lib["INVALID_VOTEE"].format(player_id=votee_id), voter_id)
      return

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
            self.stunned |= set(self.vengeance.venges)

          elif idiot_vengeance == "KILL":
            return self.__dusk(target_id) # Go to dusk, don't kill idiot yet
          elif idiot_vengeance == "WIN":
            return self.__win("IDIOT", target_id)

      self.__eliminate(actor_id, target_id)

    else:
      msg += resp_lib['ELECT_NOKILL']
      
    # Check if goons should be stunned
    if (self.rules[MRules.goon_potence] == "OFF" or 
       (self.rules[MRules.goon_potence] == "ON" and target_id == NOTARGET)):
      for goon_id in [p for p in self.players if self.players[p].role == "GOON"]:
        self.stunned |= {goon_id}

    self.cast_main(msg)
    self.__night()

  def __eliminate(self, actor_id, target_id):
    role = self.players[target_id].role
    reveal = dispRole(role, self.rules[MRules.reveal_on_death])

    msg = resp_lib["ELIMINATE"].format(target=target_id, role=reveal)
    self.cast_main(msg)

    del self.players[target_id]

    for p,contract in self.contracts.items():
      if contract.charge == target_id:
        # charge has died
        if contract.role == "AGENT":
          self.send_dm(resp_lib["CHARGE_DIE_GUARD"].format(charge=target_id, aggressor=actor_id), p)
          contract.success = True
        elif contract.role =="GUARD":
          self.send_dm(resp_lib["CHARGE_DIE_AGENT"].format(charge=target_id, aggressor=actor_id), p)
          contract.success = False
        elif contract.role == "SURVIVOR":
          self.send_dm(resp_lib["SURVIVOR_DIE"].format(aggressor=actor_id), p)
        # Refocus if charge role is still alive
        if p in self.players and (
          (contract.role == "GUARD" and self.rules[MRules.charge_refocus_guard]) or
          (contract.role == "AGENT" and self.rules[MRules.charge_refocus_agent])):
          self.__refocus(p, target_id, actor_id, contract.role)

    n_players = len(self.players)
    n_mafia = len([p for p in self.players.values() if p.role.is_mafia()])
    if n_mafia == 0:
      self.__win("Town")
      return
    elif n_mafia>= (n_players+1) // 2:
      self.__win("Mafia")
      return

    msg = resp_lib["SHOW_ROLES"].format(self.start_roles)
    self.send_dm(msg, target_id)

  def __refocus(self, actor, target, aggressor, role):

    new_charge = aggressor
    if role == "GUARD":
      if actor == aggressor or not aggressor in self.players:
        new_role = "IDIOT"
        new_charge = actor
      else:
        new_role = "AGENT"
    elif role == "AGENT":
      if actor == aggressor or not aggressor in self.players:
        new_role = "SURVIVOR"
        new_charge = actor
      else:
        new_role = "GUARD"

    msg = resp_lib["REFOCUS"].format(new_role=new_role)
    if new_role in ("GUARD","AGENT"):
       msg += "\n" + resp_lib["CHARGE"].format(charge=new_charge)

    self.players[actor].role = new_role
    contract = self.contracts[actor]
    contract.role = new_role
    contract.charge = new_charge
    contract.success = new_role in ("SURVIVOR","GUARD")

  def __night(self):
    msg = resp_lib['NIGHT']
    self.cast_main(msg)

    for p in self.stunned:
      self.send_dm(resp_lib["STUN"], p)

    opts = resp_lib['NIGHT_OPTIONS']
    opts += '\n'.join(listMenu(self.players))
    for t_p in [p for p in self.players if self.players[p].role.is_targeting()]:
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
    
    # Check phase
    if not self.phase == MPhase.NIGHT:
      self.send_dm(resp_lib["INVALID_TARGET_PHASE"], actor_id)
      return
    # Check valid actor
    if not actor_id in self.players:
      self.send_dm(resp_lib["INVALID_TARGETER"], actor_id)
      return
    if not self.players[actor_id].role.is_targeting():
      self.send_dm(resp_lib["INVALID_TARGET_ROLE"], actor_id)
    # Check stunned
    if actor_id in self.stunned and not target_id == NOTARGET:
      self.send_dm(resp_lib["INVALID_TARGET_STUNNED"], actor_id)
    # Check valid target
    if not (target_id in self.players or target_id == NOTARGET):
      self.send_dm(resp_lib["INVALID_TARGETED"].format(target_id=target_id), actor_id)
      return

    return self.__target(actor_id, target_id)

  def __target(self, actor_id : MPlayerID, target_id : Optional[MPlayerID]):
    actor = self.players[actor_id]
    actor.target = target_id

    if actor.target == NOTARGET:
      msg = resp_lib["NOTARGET"]
    else:
      msg = resp_lib["TARGET"].format(target=actor.target)
    self.send_dm(msg, actor.id)

    if (self.mafia_target != None) and all([p.target != None for p in self.players.values() if p.role.is_targeting()]):
      self.__dawn()
    return

  def mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    # TODO: validation
    # Check phase
    if not self.phase == MPhase.NIGHT:
      self.cast_mafia(resp_lib["INVALID_TARGET_PHASE"])
      return
    # Check valid actor
    if not targeter_id in self.players:
      self.cast_mafia(resp_lib["INVALID_TARGETER"])
      return
    if not self.players[targeter_id].role.is_mafia():
      self.cast_mafia(resp_lib["INVALID_TARGET_ROLE"])
      return
    # Check stunned
    if targeter_id in self.stunned and not target_id == NOTARGET:
      self.cast_mafia(resp_lib["INVALID_TARGET_STUNNED"])
      return
    # Check valid target
    if not (target_id in self.players or target_id == NOTARGET):
      self.cast_mafia(resp_lib["INVALID_TARGETED"].format(target_id=target_id))
      return
    return self.__mtarget(targeter_id, target_id)

  def __mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.mafia_target = (target_id, targeter_id)
    msg = resp_lib["MTARGET"].format(actor=targeter_id, target=target_id)
    self.cast_mafia(msg)
    if (self.mafia_target != None) and all([p.target != None for p in self.players.values() if p.role.is_targeting()]):
      self.__dawn()
    return

  def __dawn(self):
    main_msg = resp_lib['DAWN']

    self.__dawn_strip()
    target_saved = self.__dawn_save()
    main_msg += self.__kill(target_saved)
    main_msg += self.__dawn_milk()
    self.__dawn_investigate()

    self.cast_main(main_msg)

    self.day += 1
    return self.__day()

  def __dawn_strip(self):
    for stripper_id in [p for p in self.players if self.players[p].role == "STRIPPER"]:
      stripper = self.players[stripper_id]
      if not stripper.target in (NOTARGET, None):
        self.stripped.append(stripper.target)
        _know_if_stripped = self.rules[MRules.know_if_stripped]
        target = self.players[stripper.target]
        msg = resp_lib["STRIP"]
        if _know_if_stripped == "ON":
          self.send_dm(msg, target.id)
        elif _know_if_stripped == "TARGET":
          if target.role.is_targeting() or target.role == "CELEB":
            self.send_dm(msg, target.id)

  def __dawn_save(self) -> bool:
    target_saved = False
    if not self.mafia_target[0] in (NOTARGET, None):
      for doctor_id in [p for p in self.players if self.players[p].role == "DOCTOR"]:
        doctor = self.players[doctor_id]
        if not doctor.target in (NOTARGET, None):
          success = doctor.target == self.mafia_target[0]
          is_stripped = doctor_id in self.stripped

          if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
            self.send_dm(resp_lib["STRIP"], doctor_id)

          if success and not is_stripped:
            target_saved = True
            if self.rules[MRules.know_if_saved_doc] == "ON":
              self.send_dm(resp_lib["SAVE_DOC"], doctor_id)
    return target_saved

  def __kill(self, target_saved:bool) -> str:
    main_msg = ""
    if self.mafia_target[0] in (NOTARGET, None):
      main_msg += "\n" + resp_lib["KILL_FAIL_QUIET"]
    else:
      if target_saved:
        if self.rules[MRules.know_if_saved] == "OFF":
          main_msg += "\n" + resp_lib["KILL_FAIL_QUIET"]
        elif self.rules[MRules.know_if_saved] == "SECRET":
          main_msg += "\n" + resp_lib["SAVE_SECRET"]
        elif self.rules[MRules.know_if_saved] == "SAVED":
          main_msg += "\n" + resp_lib["SAVE"].format(target=self.mafia_target[0])
        if self.rules[MRules.know_if_saved_self] == "ON":
          self.send_dm(resp_lib["SAVE_SELF"])
      else:
        main_msg += "\n" + resp_lib["KILL"].format(target=self.mafia_target[0])
        self.__eliminate(self.mafia_target[1], self.mafia_target[0])
    return main_msg

  def __dawn_milk(self) -> str:
    main_msg = ""
    for milky_id in [p for p in self.players if self.players[p].role == "MILKY"]:
      milky = self.players[milky_id]
      if not milky.target in (NOTARGET, None):
        is_stripped = milky_id in self.stripped
        success = milky.target in self.players

        if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
          self.send_dm(resp_lib["STRIP"], milky_id)

        if not is_stripped and success:
          main_msg += "\n" + resp_lib["MILK"].format(target=milky.target)
    return main_msg

  def __dawn_investigate(self):
    for cop_id in [p for p in self.players if self.players[p].role == "COP"]:
      cop = self.players[cop_id]
      if not cop.target in (NOTARGET, None):
        is_stripped = cop_id in self.stripped
        success = cop.target in self.players

        if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
          self.send_dm(resp_lib["STRIP"], cop_id)
        
        if not is_stripped and success:
          investigation = cop.target.role.investigate(self.rules.cop_strength)
          self.send_dm(resp_lib["INVESTIGATE"].format(target=cop.target, role=investigation))
  
  def __day(self):
    self.phase = MPhase.DAY
    self.mafia_target = None
    for p in self.players.values():
      p.target = None
    self.stunned = set()

  def __dusk(self, idiot_id):
    self.phase = MPhase.DUSK
    self.cast_main(resp_lib["DUSK"])
    opts = resp_lib["DUSK_OPTIONS"]
    opts += "\n".join(listMenu(self.vengeance.venges, notarget=False))
    self.send_dm(opts, self.vengeance.idiot)

  def itarget(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    # Check Phase and player
    if not self.phase == MPhase.DUSK:
      self.send_dm(resp_lib["INVALID_ITARGET_PHASE"], idiot_id)
      return
    if not (self.vengeance != None and idiot_id == self.vengeance.idiot):
      self.send_dm(resp_lib["INVALID_ITARGET_PLAYER"])
      return
    # Check that the target is valid
    if self.vengeance != None and target_id in self.vengeance.venges:
      self.send_dm(resp_lib["INVALID_ITARGETED"])
      return
    return self.__itarget(idiot_id, target_id)
    
  def __itarget(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.__vengeance(idiot_id, target_id)

  def __vengeance(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    msg = resp_lib['VENGEANCE'].format(actor=idiot_id, target=target_id)
    self.cast_main(msg)
    self.__eliminate(idiot_id, target_id)
    self.__eliminate(self.vengeance.final_vote, idiot_id)
    self.__night()

  def __contract_result(self, contractor_id:MPlayerID, contract:MContract):
    if contract.success:
      return resp_lib["CONTRACT_WIN"].format(role=contract.role, player=contractor_id, charge=contract.charge)
    else:
      return resp_lib["CONTRACT_LOSE"].format(role=contract.role, player=contractor_id, charge=contract.charge)

  def __win(self, team, extra=None):

    if team == "IDIOT": # Nobody but Idiot wins TODO: switch all contracts to loss?
      msg = resp_lib["IDIOT_WIN"].format(idiot=extra)
      msg += "\n" + resp_lib["SHOW_ROLES"].format(self.start_roles)
      raise IdiotWinException(extra, msg)

    msg = resp_lib["WIN"].format(winning_team=team)

    for p_id,contract in self.contracts:
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += "\n" + resp_lib["SHOW_ROLES"].format(self.start_roles)

    raise TeamWinException(team, msg)

  def __repr__(self):
    msg = "MState:\n"
    msg += "{} {}\n  ".format(repr(self.phase), self.day)
    msg += "\n  ".join([repr(p) for p in self.players.values()])
    return msg

  def to_json(self):
    d = {}
    for name in ["id","day","phase","players","contracts","start_roles","rules"]:
      d[name] = self.__dict__[name]
    return d

  @staticmethod
  def from_json(d):
    mstate = MState(d['id'],d['rules'])
    mstate.day = d['day']
    mstate.phase = d['phase']
    mstate.players = d['players']
    mstate.contracts = d['contracts']
    mstate.start_roles = d['start_roles']
    return mstate