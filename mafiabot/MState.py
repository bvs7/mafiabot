from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable, Any, Tuple, Set
from threading import Lock, Thread
import json
from collections import OrderedDict

from .MInfo import *
from .MPlayer import MPlayer, MPlayerID, NOTARGET
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

class InvalidActionException(Exception):
  def __init__(self, msg):
    self.msg = msg
    super().__init__(msg)

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
    self.revealed : Set[MPlayerID] = set() # Keep track of which CELEBs have revealed.
    self.vengeance : Optional[MVengeance] = None # Used when an IDIOT needs to get revenge.

    self.start_roles = "Init"

  def start(self, ids, roles, contracts):
    # Check inputs for validity...
    # Ensure starting roles are valid?
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

    msg = resp_lib["START"]

    for p in self.players:
      msg += "\n" + ("[%s]"%p)

    known_roles = self.rules[MRules.known_roles]
    role_list = dispKnownRoles(makeRoleDict(roles), known_roles)
    msg += "\n" + role_list

    maf_msg = resp_lib["START_MAFIA"]
    maf_players = [p for p in self.players if self.players[p].role.is_mafia()]
    if len(maf_players) > 1:
      for p in maf_players:
        maf_msg += "\n" + "[{}]: {}".format(p, self.players[p].role)

    self.cast_main(msg)
    self.cast_mafia(maf_msg)
    self.day = 1
    start_night = self.rules[MRules.start_night]
    even = len(self.players) % 2 == 0
    to_night = (  (start_night == "ON") or
      (start_night == "EVEN" and even) or
      (start_night == "ODD" and not even))
    if to_night:
      self.__night()
    else:
      self.__day()

  def reveal(self, reveal_id : MPlayerID):
    if not reveal_id in self.players:
      raise InvalidActionException(resp_lib["INVALID_REVEAL_PLAYER"])
    p = self.players[reveal_id]
    if not p.role == MRole.CELEB:
      raise InvalidActionException(resp_lib["INVALID_REVEAL_ROLE"])
    if not self.phase == MPhase.DAY:
      raise InvalidActionException(resp_lib["INVALID_REVEAL_PHASE"])
    self.__reveal(reveal_id)
  
  def __reveal(self, reveal_id):
    p = self.players[reveal_id]
    if not reveal_id in self.revealed:
      if reveal_id in self.stripped:
        if self.rules[MRules.know_if_stripped] == "USEFUL": 
          self.send_dm(resp_lib["STRIPPED"], reveal_id)
      else:
        self.revealed.add(reveal_id)
        msg = resp_lib['REVEAL'].format(actor=p.id, role=p.role)
        self.cast_main(msg)
    else:
      msg = resp_lib['REVEAL_REMINDER'].format(actor=p.id, role=p.role)
      self.cast_main(msg)

  def vote(self, voter_id : MPlayerID, votee_id : Optional[MPlayerID]):
    if not self.phase == MPhase.DAY:
      raise InvalidActionException(resp_lib["INVALID_VOTE_PHASE"])
    if not voter_id in self.players:
      msg = resp_lib["INVALID_VOTER"].format(player_id=voter_id)
      raise InvalidActionException(msg)
    if not (votee_id in self.players or votee_id in (NOTARGET, None)):
      msg = resp_lib["INVALID_VOTEE"].format(player_id=votee_id)
      raise InvalidActionException(msg)
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
    try:
      main_msg = [""]
      nokill = False
      if not target_id == NOTARGET:
        target = self.players[target_id]

        main_msg[0] += resp_lib['ELECT'].format(target=target_id)

        if target.role == "IDIOT":
          self.contracts[target_id].success = True
          self.vengeance = MVengeance([p_id for p,p_id in self.players.items() if p.vote == target_id], actor_id, target_id)

          idiot_vengeance = self.rules[MRules.idiot_vengeance]
          if not idiot_vengeance == "OFF":
            main_msg[0] += resp_lib["ELECT_IDIOT"]

            if idiot_vengeance == "DAY":
              main_msg[0] += resp_lib["ELECT_DAY"]
              self.cast_main(main_msg[0])
              return self.__day()

            elif idiot_vengeance == "STUN":
              main_msg[0] += resp_lib["ELECT_STUN"]
              self.stunned |= set(self.vengeance.venges)

            elif idiot_vengeance == "KILL":
              return self.__dusk(target_id) # Go to dusk, don't kill idiot yet
            elif idiot_vengeance == "WIN":
              return self.__idiot_win(target_id, main_msg)

        self.__eliminate(actor_id, target_id, main_msg)

      else:
        main_msg[0] += resp_lib['ELECT_NOKILL']
        nokill = True

    except EndGameException as e:
      self.cast_main(main_msg[0])
      raise e

    main_msg[0] += '\n'
    self.__night(main_msg, nokill)

  def __eliminate(self, actor_id, target_id, main_msg:List[str]) -> str:
    role = self.players[target_id].role
    reveal = dispRole(role, self.rules[MRules.reveal_on_death])

    main_msg[0] += "\n"
    main_msg[0] += resp_lib["ELIMINATE"].format(target=target_id, role=reveal)

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
      self.__team_win(MTeam.Town, main_msg)
    elif n_mafia>= (n_players+1) // 2:
      self.__team_win(MTeam.Mafia, main_msg)

    role_msg = resp_lib["SHOW_ROLES"].format(self.start_roles)
    self.send_dm(role_msg, target_id)

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

  def __night(self, main_msg:List[str]=[""], nokill=False):
    main_msg[0] += resp_lib['NIGHT']
    self.cast_main(main_msg[0])

    # Check if goons should be stunned
    if (self.rules[MRules.goon_potence] == "OFF" or 
       (self.rules[MRules.goon_potence] == "ON" and not nokill)):
      for goon_id in [p for p in self.players if self.players[p].role == MRole.GOON]:
        self.stunned |= {goon_id}

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
    if not self.phase == MPhase.NIGHT:
      raise InvalidActionException(resp_lib["INVALID_TARGET_PHASE"])
    if not actor_id in self.players:
      raise InvalidActionException(resp_lib["INVALID_TARGETER"])
    if not self.players[actor_id].role.is_targeting():
      raise InvalidActionException(resp_lib["INVALID_TARGET_ROLE"])
    if actor_id in self.stunned and not target_id == NOTARGET:
      raise InvalidActionException(resp_lib["INVALID_TARGET_STUNNED"])
    if not (target_id in self.players or target_id == NOTARGET):
      msg = resp_lib["INVALID_TARGETED"].format(target_id=target_id)
      raise InvalidActionException(msg)
    if (self.rules[MRules.no_milk_self] == "ON" and 
        self.players[actor_id].role == MRole.MILKY and actor_id==target_id):
      raise InvalidActionException(resp_lib["INVALID_TARGET_MILK_SELF"])
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
    if not self.phase == MPhase.NIGHT:
      raise InvalidActionException(resp_lib["INVALID_TARGET_PHASE"])
    if not targeter_id in self.players:
      raise InvalidActionException(resp_lib["INVALID_TARGETER"])
    if not self.players[targeter_id].role.is_mafia():
      raise InvalidActionException(resp_lib["INVALID_TARGET_ROLE"])
    if targeter_id in self.stunned and not target_id == NOTARGET:
      raise InvalidActionException(resp_lib["INVALID_TARGET_STUNNED"])
    if not (target_id in self.players or target_id == NOTARGET):
      msg = resp_lib["INVALID_TARGETED"].format(target_id=target_id)
      raise InvalidActionException(msg)
    return self.__mtarget(targeter_id, target_id)

  def __mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.mafia_target = (target_id, targeter_id)
    msg = resp_lib["MTARGET"].format(actor=targeter_id, target=target_id)
    self.cast_mafia(msg)
    if (self.mafia_target != None) and all([p.target != None for p in self.players.values() if p.role.is_targeting()]):
      self.__dawn()
    return

  def __dawn(self):
    main_msg = [resp_lib['DAWN']]

    self.__dawn_strip()
    target_saved = self.__dawn_save()
    self.__kill(main_msg, target_saved)
    self.__dawn_milk(main_msg)
    self.__dawn_investigate()

    self.cast_main(main_msg[0])

    self.day += 1
    return self.__day()

  def __dawn_strip(self):
    for stripper_id in [p for p in self.players if self.players[p].role == "STRIPPER"]:
      stripper = self.players[stripper_id]
      if not stripper.target in (NOTARGET, None):
        self.stripped.append(stripper.target)
        _know_if_stripped = self.rules[MRules.know_if_stripped]
        target = self.players[stripper.target]
        msg = resp_lib["STRIPPED"]
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
            self.send_dm(resp_lib["STRIPPED"], doctor_id)

          if success and not is_stripped:
            target_saved = True
            if self.rules[MRules.know_if_saved_doc] == "ON":
              self.send_dm(resp_lib["SAVE_DOC"], doctor_id)
    return target_saved

  def __kill(self, main_msg:List[str], target_saved:bool) -> str:
    if self.mafia_target[0] in (NOTARGET, None):
      main_msg[0] += "\n" + resp_lib["KILL_FAIL_QUIET"]
    else:
      if target_saved:
        if self.rules[MRules.know_if_saved] == "OFF":
          main_msg[0] += "\n" + resp_lib["KILL_FAIL_QUIET"]
        elif self.rules[MRules.know_if_saved] == "SECRET":
          main_msg[0] += "\n" + resp_lib["SAVE_SECRET"]
        elif self.rules[MRules.know_if_saved] == "SAVED":
          main_msg[0] += "\n" + resp_lib["SAVE"].format(target=self.mafia_target[0])
        if self.rules[MRules.know_if_saved_self] == "ON":
          self.send_dm(resp_lib["SAVE_SELF"],self.mafia_target[0])
      else:
        main_msg[0] += "\n" + resp_lib["KILL"].format(target=self.mafia_target[0])
        try:
          self.__eliminate(self.mafia_target[1], self.mafia_target[0], main_msg)
        except EndGameException as e:
          self.cast_main(main_msg[0])
          raise e

  def __dawn_milk(self, main_msg:List[str]) -> str:
    for milky_id in [p for p in self.players if self.players[p].role == "MILKY"]:
      milky = self.players[milky_id]
      if not milky.target in (NOTARGET, None):
        is_stripped = milky_id in self.stripped
        success = milky.target in self.players

        if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
          self.send_dm(resp_lib["STRIPPED"], milky_id)

        if not is_stripped and success:
          main_msg[0] += "\n" + resp_lib["MILK"].format(target=milky.target)

  def __dawn_investigate(self):
    for cop_id in [p for p in self.players if self.players[p].role == "COP"]:
      cop = self.players[cop_id]
      if not cop.target in (NOTARGET, None):
        is_stripped = cop_id in self.stripped
        success = cop.target in self.players

        if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
          self.send_dm(resp_lib["STRIPPED"], cop_id)
        
        if not is_stripped and success:
          investigation = self.players[cop.target].role.investigate(self.rules[MRules.cop_strength])
          self.send_dm(resp_lib["INVESTIGATE"].format(target=cop.target, role=investigation), cop_id)
  
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
    main_msg = [resp_lib['VENGEANCE'].format(actor=idiot_id, target=target_id)]
    try:
      self.__eliminate(idiot_id, target_id, main_msg) # Eliminate target before idiot
      self.__eliminate(self.vengeance.final_vote, idiot_id, main_msg)
    except EndGameException as e:
      self.cast_main(main_msg[0])
      raise e
    self.cast_main(main_msg[0])
    self.__night()

  def __contract_result(self, contractor_id:MPlayerID, contract:MContract):
    if contract.success:
      return resp_lib["CONTRACT_WIN"].format(role=contract.role, player=contractor_id, charge=contract.charge)
    else:
      return resp_lib["CONTRACT_LOSE"].format(role=contract.role, player=contractor_id, charge=contract.charge)

  def __team_win(self, team, main_msg:List[str]):
    
    msg = "\n" + resp_lib["WIN"].format(winning_team=team)
    for p_id,contract in self.contracts:
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += "\n" + resp_lib["SHOW_ROLES"].format(self.start_roles)
    main_msg[0] += msg
    raise TeamWinException(team, main_msg[0])

  def __idiot_win(self, idiot, main_msg:List[str]):
    msg = "\n" + resp_lib["IDIOT_WIN"].format(idiot)
    for p_id,contract in self.contracts:
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += resp_lib["SHOW_ROLES"].format(self.start_roles)
    main_msg[0] += msg
    raise IdiotWinException(idiot, msg)

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