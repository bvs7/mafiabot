from enum import Enum, auto
from typing import List, Dict, Optional, Set, Iterable, Tuple
from threading import Lock, Thread
import json
from collections import OrderedDict

from ..resp_lib import get_resp, resp_lib
from .MInfo import dispKnownRoles, makeRoleDict, createStartRolesMsg, dispRole
from .MRole import MRole, MTeam
from .MPlayer import MPlayer, MPlayerID, NOTARGET
from .MRules import MRules
from ..util import VEnum
from .MRoleGen import MAssignment, MRoleGenType, MContract

from ..chatinterface import MChat, MDM, TestMChat

__all__ = ['MState','MPhase', 'InvalidActionException', 'EndGameException','IdiotWinException','TeamWinException']

# TODO: Generalize listMenu? Make that a DM option?

class MPhase(VEnum):
  INIT = auto()
  DAY = auto()
  NIGHT = auto()
  DUSK = auto()
  END = auto()

  def active(self):
    return not self in {MPhase.INIT, MPhase.END}

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
  def __init__(self, msg):
    self.msg = msg
    super().__init__(self)

class IdiotWinException(EndGameException):
  def __init__(self, idiot_id, msg=None):
    self.idiot_id = idiot_id
    if msg == None:
      msg = get_resp('IDIOT_WIN',idiot=idiot_id)
    super().__init__(msg)

class TeamWinException(EndGameException):
  def __init__(self, team, msg=None):
    self.team = team
    if msg == None:
      msg = get_resp('WIN', winning_team=team)
    super().__init__(msg)

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

def check_end(func):
  def inner(self, *args):
    if self.phase == MPhase.END:
      raise InvalidActionException(resp_lib["INVALID_ACTION_END"])
    return func(self, *args)
  return inner

class MState:

  def __init__(self, main_chat=MChat(), mafia_chat=MChat(), dms=MDM(), rules:MRules=MRules()):

    self.main_chat = main_chat
    self.mafia_chat = mafia_chat
    self.dms = dms

    self.halt_timer = self.__halt_timer # Called when time progresses to stop timer?

    self.rules = rules

    self.day = 0
    self.phase = MPhase.INIT

    self.players : Dict[MPlayerID, MPlayer] = OrderedDict() # maps players to playerids
    self.contracts : Dict[MPlayerID, MContract] = {} #Dicts? player_id -> (role, target, success)

    self.mafia_target : Optional[Tuple[MPlayerID, MPlayerID]] = None
    self.stripped : Set[MPlayerID] = set() # Keep track of who is stripped through dawn calc and during next day (celeb).
    self.stunned : Set[MPlayerID] = set() # Keep track of who is stunned (GOON, or by IDIOT) at night.
    self.revealed : Set[MPlayerID] = set() # Keep track of which CELEBs have revealed.
    self.vengeance : Optional[MVengeance] = None # Used when an IDIOT needs to get revenge.

    self.main_msg = ""

    self.start_roles = "Init"

  def destroy(self):
    print("DEL MSTATE")

  # Is Iterable what we want? or would some other base class be better?
  def start(self, assignments:Iterable[MAssignment], 
                  contracts:Dict[MPlayerID, MContract]):
    # Check inputs for validity...
    
    assert isinstance(assignments, Iterable)
    assert len(assignments) >= 3
    assert isinstance(assignments[0][0],type(NOTARGET))
    assert isinstance(assignments[0][1], MRole) or isinstance(assignments[0][1],str)

    assignments = [(i,MRole(r)) for i,r in assignments]
    # Ensure starting roles are valid?
    maf = [m for (i,m) in assignments if m.is_mafia()]
    n_players = len(assignments)
    n_mafia = len(maf)
    assert not (n_mafia >= (n_players+1) // 2)
    assert not (n_mafia == 0)
    # Ensure starting contracts are valid
    for p_id, contract in contracts.items():
      assert (p_id, contract.role) in assignments
      assert contract.charge in [i for (i,r) in assignments]
      assert contract.success == (contract.role in {MRole.SURVIVOR, MRole.GUARD})

    self.__start(assignments, contracts)

  def __start(self, assignments:Iterable[MAssignment],
                    contracts:Dict[MPlayerID, MContract]):
    self.contracts = contracts
    self.players_init(assignments)
    self.send_role_expl()
    self.cast_start_msgs()
    
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

  def __halt_timer(self, *args):
    if self.halt_timer == self.__halt_timer:
      raise NotImplementedError
    else:
      self.halt_timer(*args)

  @check_end
  def timer(self):
    # Timer triggers, forcing forward progress in the game state
    self.__timer()
    # how does a timer get disabled when the game progresses normally?
    # Right now, with halt_timer callback. Or have MState handle timers? nah
  
  def __timer(self): # TODO: Testing
    self.main_msg = ""
    if self.phase == MPhase.DAY:
      self.main_msg += resp_lib["TIMER_DAY"] + "\n"
      self.__night(nokill=True)
    elif self.phase == MPhase.NIGHT:
      self.main_msg += resp_lib["TIMER_NIGHT"] + "\n"
      self.__dawn()
    elif self.phase == MPhase.DUSK:
      idiot_id=self.vengeance.idiot
      if not self.vengeance.final_vote == self.vengeance.idiot:
        other_id = self.vengeance.final_vote
      else:
        other_id = None
      self.main_msg += resp_lib["TIMER_DUSK"].format(**locals())
      if not other_id == None:
        self.__eliminate(idiot_id, other_id)
      self.__eliminate(self.vengeance.venges[-1], self.vengeance.idiot)
      self.main_msg += "\n"
      self.__night(nokill=False)

  @check_end
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
          self.dms.send(resp_lib["STRIPPED"], reveal_id)
      else:
        self.revealed.add(reveal_id)
        self.main_chat.cast_resp('REVEAL',actor=reveal_id, role=p.role)
    else:
      self.main_chat.cast_resp('REVEAL_REMINDER',actor=reveal_id, role=p.role)

  @check_end
  def vote(self, voter_id : MPlayerID, votee_id : Optional[MPlayerID]):
    if not self.phase == MPhase.DAY:
      raise InvalidActionException(resp_lib["INVALID_VOTE_PHASE"])
    if not voter_id in self.players:
      msg = get_resp("INVALID_VOTER", player_id=voter_id)
      raise InvalidActionException(msg)
    if not (votee_id in self.players or votee_id in (NOTARGET, None)):
      msg = get_resp("INVALID_VOTEE", player_id=votee_id)
      raise InvalidActionException(msg)
    return self.__vote(voter_id, votee_id)

  def __vote(self, voter_id : MPlayerID, votee_id : Optional[MPlayerID]):
    players = self.players
    voter = players[voter_id]
    f_votee_id = voter.vote
    # TODO: Confirm vote if it is the same?
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
      msg += get_resp('VOTE', voter=voter_id, votee=votee_id)
    else:
      if votee_id == None:
        msg += get_resp('VOTE_RETRACT', voter=voter_id, f_votee=f_votee_id)
      else:
        msg += get_resp('VOTE_CHANGE', voter=voter_id, votee=votee_id, f_votee=f_votee_id)

    if votee_id in players:
      msg += get_resp('VOTE_UPDATE', votee=votee_id, n_voters=n_voters, thresh=thresh)
    elif votee_id == NOTARGET:
      msg += get_resp('VOTE_UPDATE_NOKILL', n_voters=n_voters, nokill_thresh=nokill_thresh)

    self.main_chat.cast(msg)

    # Check for end of DAY
    if votee_id == None:
      return

    over_nokill_thresh = votee_id == NOTARGET and n_voters >= nokill_thresh
    over_thresh = votee_id != NOTARGET and n_voters >= thresh

    if over_nokill_thresh or over_thresh:
      self.__elect(voter_id, votee_id)
      return
  
  def __elect(self, actor_id, target_id):
    self.main_msg = ""
    nokill = False
    if not target_id == NOTARGET:
      target = self.players[target_id]

      self.main_msg += get_resp('ELECT',target=target_id)

      if target.role == "IDIOT":
        self.contracts[target_id].success = True
        voters = [p_id for (p_id,p) in self.players.items() if (
          p.vote == target_id and p_id != target_id)]
        self.vengeance = MVengeance(voters, actor_id, target_id)

        idiot_vengeance = self.rules[MRules.idiot_vengeance]
        if not idiot_vengeance == "OFF":
          self.main_msg += resp_lib["ELECT_IDIOT"]

          if idiot_vengeance == "DAY":
            self.main_msg += resp_lib["ELECT_DAY"]
            self.main_chat.cast(self.main_msg)
            self.main_msg = ""
            return self.__day()

          elif idiot_vengeance == "STUN":
            self.main_msg += resp_lib["ELECT_STUN"]
            self.stunned |= set(self.vengeance.venges)

          elif idiot_vengeance == "KILL":
            return self.__dusk(target_id) # Go to dusk, don't kill idiot yet
          elif idiot_vengeance == "WIN":
            return self.__idiot_win(target_id)

      self.__eliminate(actor_id, target_id)

    else:
      self.main_msg += resp_lib['ELECT_NOKILL']
      nokill = True

    self.main_msg += '\n'
    self.__night(nokill)

  def __eliminate(self, actor_id, target_id) -> str:
    role = self.players[target_id].role
    reveal = dispRole(role, self.rules[MRules.reveal_on_death])

    self.main_msg += "\n" + get_resp("ELIMINATE",target=target_id, role=reveal)

    del self.players[target_id]

    for p,contract in self.contracts.items():
      if contract.charge == target_id:
        # charge has died
        dm_msg = [""]
        if contract.role == "AGENT":
          dm_msg[0] += get_resp("CHARGE_DIE_AGENT",charge=target_id, aggressor=actor_id)
          contract.success = True
        elif contract.role =="GUARD":
          dm_msg[0] += get_resp("CHARGE_DIE_GUARD",charge=target_id, aggressor=actor_id)
          contract.success = False
        elif contract.role == "SURVIVOR":
          dm_msg[0] += get_resp("SURVIVOR_DIE",aggressor=actor_id)
          contract.success = False
        # Refocus if charge role is still alive
        if p in self.players and (
          (contract.role == "GUARD" and self.rules[MRules.charge_refocus_guard]) or
          (contract.role == "AGENT" and self.rules[MRules.charge_refocus_agent])):
          self.__refocus(p, target_id, actor_id, contract.role, dm_msg)
        if not dm_msg[0] == "":
          self.dms.send(dm_msg[0], p)

    n_players = len(self.players)
    n_mafia = len([p for p in self.players.values() if p.role.is_mafia()])
    if n_mafia == 0:
      self.__team_win(MTeam.Town)
    elif n_mafia>= (n_players+1) // 2:
      self.__team_win(MTeam.Mafia)

    role_msg = get_resp("SHOW_ROLES",start_roles=self.start_roles)
    self.dms.send(role_msg, target_id)

  # TODO: when the game is over, don't refocus?
  def __refocus(self, actor, target, aggressor, role, dm_msg:List[str]=[""]):
    new_charge = aggressor
    if role == MRole.GUARD:
      if actor == aggressor or not aggressor in self.players:
        new_role = MRole.IDIOT
        new_charge = actor
      else:
        new_role = MRole.AGENT
    elif role == MRole.AGENT:
      if actor == aggressor or not aggressor in self.players:
        new_role = MRole.SURVIVOR
        new_charge = actor
      else:
        new_role = MRole.GUARD

    dm_msg[0] += "\n" + get_resp("REFOCUS",new_role=new_role)
    if new_role in (MRole.GUARD, MRole.AGENT):
       dm_msg[0] += "\n" + get_resp("CHARGE_ASSIGN",charge=new_charge)

    self.players[actor].role = new_role
    contract = self.contracts[actor]
    contract.role = new_role
    contract.charge = new_charge
    contract.success = new_role in ("SURVIVOR","GUARD")

    # Modify start roles:
    srs = self.start_roles.split('\n')
    goal = '[{}]:'.format(actor)
    for i,sr in enumerate(srs):
      if sr[:len(goal)] == goal:
        srs[i] +=  " -> {role}".format(role=new_role)
        if new_role in {MRole.GUARD,MRole.AGENT}:
          srs[i] += "([{charge}])".format(charge=new_charge)
    self.start_roles = "\n".join(srs)

  def __night(self, nokill=False):
    self.__halt_timer()

    self.main_msg += resp_lib['NIGHT']
    self.main_chat.cast(self.main_msg)
    self.main_msg = ""

    # Check if goons should be stunned
    if (self.rules[MRules.goon_potence] == "OFF" or 
       (self.rules[MRules.goon_potence] == "ON" and not nokill)):
      for goon_id in [p for p in self.players if self.players[p].role == MRole.GOON]:
        self.stunned |= {goon_id}

    for p in self.stunned:
      self.dms.send(resp_lib["STUN"], p)

    opts = resp_lib['NIGHT_OPTIONS']
    opts += '\n'.join(self.listMenu(self.players.keys()))
    for t_p in [p for p in self.players if self.players[p].role.is_targeting()]:
      msg = opts
      if t_p in self.stunned:
        msg = resp_lib["STUNNED"] + "\n" + opts
      self.dms.send(msg, t_p)
    self.mafia_chat.cast(opts)

    self.phase = MPhase.NIGHT
    for player in self.players.values():
      player.vote = None
    self.stripped = set()
    self.vengeance = None

  @check_end
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
      msg = get_resp("INVALID_TARGETED",target_id=target_id)
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
      msg = get_resp("TARGET",target=actor.target)
    self.dms.send(msg, actor_id)

    if (self.mafia_target[0] != None) and all([p.target != None for p in self.players.values() if p.role.is_targeting()]):
      self.__dawn()
    return

  @check_end
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
      msg = get_resp("INVALID_TARGETED",target_id=target_id)
      raise InvalidActionException(msg)
    return self.__mtarget(targeter_id, target_id)

  def __mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.mafia_target = (target_id, targeter_id)
    msg = get_resp("MTARGET",actor=targeter_id, target=target_id)
    self.mafia_chat.cast(msg)
    if (self.mafia_target[0] != None) and all([p.target != None for p in self.players.values() if p.role.is_targeting()]):
      self.__dawn()
    return

  def __dawn(self):
    self.__halt_timer()
    self.main_msg += resp_lib['DAWN']

    self.__dawn_strip()
    target_saved = self.__dawn_save()
    self.__kill(target_saved)
    self.__dawn_milk()
    self.__dawn_investigate()

    self.main_chat.cast(self.main_msg)
    self.main_msg = ""

    self.day += 1
    return self.__day()

  def __dawn_strip(self):
    for stripper_id in [p for p in self.players if self.players[p].role == "STRIPPER"]:
      stripper = self.players[stripper_id]
      if not stripper.target in (NOTARGET, None):
        target_id = stripper.target
        self.stripped.add(target_id)
        _know_if_stripped = self.rules[MRules.know_if_stripped]
        target = self.players[target_id]
        msg = resp_lib["STRIPPED"]
        if _know_if_stripped == "ON":
          self.dms.send(msg, target_id)
        elif _know_if_stripped == "TARGET":
          if target.role.is_targeting() or target.role == "CELEB":
            self.dms.send(msg, target_id)

  def __dawn_save(self) -> bool:
    target_saved = False
    if not self.mafia_target[0] in {NOTARGET, None}:
      for doctor_id in [p for p in self.players if self.players[p].role == "DOCTOR"]:
        doctor = self.players[doctor_id]
        if not doctor.target in (NOTARGET, None):
          success = doctor.target == self.mafia_target[0]
          is_stripped = doctor_id in self.stripped

          if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
            self.dms.send(resp_lib["STRIPPED"], doctor_id)

          if success and not is_stripped:
            target_saved = True
            if self.rules[MRules.know_if_saved_doc] == "ON":
              self.dms.send(resp_lib["SAVE_DOC"], doctor_id)
    return target_saved

  def __kill(self, target_saved:bool) -> str:
    if self.mafia_target[0] in (NOTARGET, None):
      self.main_msg += "\n" + resp_lib["KILL_FAIL_QUIET"]
    else:
      if target_saved:
        if self.rules[MRules.know_if_saved] == "OFF":
          self.main_msg += "\n" + resp_lib["KILL_FAIL_QUIET"]
        elif self.rules[MRules.know_if_saved] == "SECRET":
          self.main_msg += "\n" + resp_lib["SAVE_SECRET"]
        elif self.rules[MRules.know_if_saved] == "SAVED":
          self.main_msg += "\n" + get_resp("SAVE",target=self.mafia_target[0])
        if self.rules[MRules.know_if_saved_self] == "ON":
          self.dms.send(resp_lib["SAVE_SELF"],self.mafia_target[0])
      else:
        self.main_msg += "\n" + get_resp("KILL",target=self.mafia_target[0])
        self.__eliminate(self.mafia_target[1], self.mafia_target[0])

  def __dawn_milk(self) -> str:
    for milky_id in [p for p in self.players if self.players[p].role == "MILKY"]:
      milky = self.players[milky_id]
      if not milky.target in (NOTARGET, None):
        is_stripped = milky_id in self.stripped
        success = milky.target in self.players

        if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
          self.dms.send(resp_lib["STRIPPED"], milky_id)

        if not is_stripped and success:
          self.main_msg += "\n" + get_resp("MILK",target=milky.target)

  def __dawn_investigate(self):
    for cop_id in [p for p in self.players if self.players[p].role == "COP"]:
      cop = self.players[cop_id]
      if not cop.target in (NOTARGET, None):
        is_stripped = cop_id in self.stripped
        success = cop.target in self.players

        if self.rules[MRules.know_if_stripped] == "USEFUL" and success and is_stripped:
          self.dms.send(resp_lib["STRIPPED"], cop_id)
        
        if not is_stripped and success:
          investigation = self.players[cop.target].role.investigate(self.rules[MRules.cop_strength])
          self.dms.send(get_resp("INVESTIGATE",target=cop.target, role=investigation), cop_id)
  
  def __day(self):
    self.__halt_timer()
    self.phase = MPhase.DAY
    self.mafia_target = (None, None)
    for p in self.players.values():
      p.target = None
    self.stunned = set()

  def __dusk(self, idiot_id):
    self.__halt_timer()
    self.phase = MPhase.DUSK
    self.main_msg += "\n" + resp_lib["DUSK"]
    self.main_chat.cast(self.main_msg)
    self.main_msg = ""
    opts = resp_lib["DUSK_OPTIONS"]
    opts += "\n".join(self.listMenu(self.vengeance.venges, notarget=False))
    self.dms.send(opts, self.vengeance.idiot)

  @check_end
  def itarget(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    if not self.phase == MPhase.DUSK:
      raise InvalidActionException(resp_lib["INVALID_ITARGET_PHASE"])
    if not (self.vengeance != None and idiot_id == self.vengeance.idiot):
      raise InvalidActionException(resp_lib["INVALID_ITARGET_PLAYER"])
    if self.vengeance != None and not target_id in self.vengeance.venges:
      raise InvalidActionException(resp_lib["INVALID_ITARGETED"])
    return self.__itarget(idiot_id, target_id)
    
  def __itarget(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.__vengeance(idiot_id, target_id)

  def __vengeance(self, idiot_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.main_msg += get_resp('VENGEANCE',actor=idiot_id, target=target_id)
    self.__eliminate(idiot_id, target_id) # Eliminate target before idiot
    self.__eliminate(self.vengeance.final_vote, idiot_id)
    self.main_chat.cast(self.main_msg)
    self.main_msg = ""
    self.__night()

  def __contract_result(self, contractor_id:MPlayerID, contract:MContract):
    if contract.success:
      msg = get_resp("CONTRACT_WIN",role=contract.role, player=contractor_id)
    else:
      msg = get_resp("CONTRACT_LOSE",role=contract.role, player=contractor_id, charge=contract.charge)
    if contract.role in {MRole.AGENT, MRole.GUARD}:
      msg += " " + get_resp("CHARGE_REVEAL",charge=contract.charge)
    return msg


  def __team_win(self, team):
    self.phase = MPhase.END
    self.halt_timer()
    msg = "\n" + get_resp("WIN",winning_team=team)
    for p_id,contract in self.contracts.items():
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += "\n" + get_resp("SHOW_ROLES",start_roles=self.start_roles)
    self.main_msg += msg
    self.main_chat.cast(self.main_msg)
    self.main_msg = ""
    raise TeamWinException(team, msg)

  def __idiot_win(self, idiot):
    self.phase = MPhase.END
    self.halt_timer()
    msg = "\n" + get_resp("IDIOT_WIN",idiot=idiot)
    for p_id,contract in self.contracts.items():
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += get_resp("SHOW_ROLES",start_roles=self.start_roles)
    self.main_msg += msg
    self.main_chat.cast(self.main_msg)
    raise IdiotWinException(idiot, msg)

  def main_status(self):
    return "TODO" # TODO

  def mafia_status(self):
    return "TODO" # TODO

  def dm_status(self, player_id):
    return "TODO" # TODO

  def players_init(self, assignments):
    for p_id,role in assignments:
      p = MPlayer(p_id, role)
      self.players[p_id] = p

  def send_role_expl(self):
    mason_strs = set(["[{}]".format(p_id) for p_id in \
      self.players if self.players[p_id].role == MRole.MASON])
    mason_msg = get_resp("MASON_REVEAL",mason_str="\n".join(mason_strs))

    for p_id,p in self.players.items():
      msg = p.role.expl()
      if p.role in {MRole.GUARD,MRole.AGENT}:
        msg += " " + get_resp("CHARGE_ASSIGN",charge=self.contracts[p_id].charge)
      if p.role in {MRole.MASON}:
        msg += "\n" + mason_msg
      for rule in MRules.relevant_rules[p.role]:
        sett = self.rules[rule]
        msg += "\n{}|{}: {}".format(rule,sett, MRules.RULE_BOOK[rule][sett])
      self.dms.send(msg, p_id)
    self.start_roles = createStartRolesMsg(self.players,self.contracts)

  def cast_start_msgs(self):
    msg = resp_lib["START"]

    for p in self.players:
      msg += "\n" + ("[%s]"%p)
    known_roles = self.rules[MRules.known_roles]
    role_list = dispKnownRoles(makeRoleDict([p.role for p in self.players.values()]), known_roles)
    msg += "\n" + role_list

    maf_msg = resp_lib["START_MAFIA"]
    maf_players = [p for p in self.players if self.players[p].role.is_mafia()]
    if len(maf_players) > 1:
      for p in maf_players:
        maf_msg += "\n" + "[{}]: {}".format(p, self.players[p].role)

    self.main_chat.cast(msg)
    self.mafia_chat.cast(maf_msg)

  @staticmethod
  def listMenu(players, notarget=True):
    p_ids = list(players)
    if notarget:
      p_ids.append("NOTARGET")
    p_lists = []
    while len(p_ids) > 0:
      l = min(len(p_ids),26)
      p_lists.append(p_ids[:l])
      p_ids = p_ids[l:]
    ps = []
    for i,p_list in enumerate(p_lists):
      prefix = "" if i==0 else chr(ord('A')+i-1)
      c = 'A'
      for p_id in p_list:
        ps.append("{}{}: [{}]".format(prefix,c,p_id))
        c = chr(ord(c)+1)
    return ps

  def __str__(self):
    """Status request?"""
    msg = "{} {}:\n".format(self.phase, self.day)
    msg += dispKnownRoles([p.role for p in self.players.values()], self.rules[MRules.known_roles]) + "\n"

    # TODO: Display Votes
    return msg

  def __repr__(self):
    msg = "MState:\n"
    msg += "{} {}\n  ".format(repr(self.phase), self.day)
    msg += "\n  ".join([repr(p) for p in self.players.values()])
    return msg

  def to_json(self):
    d = {}
    for name in ["day","phase","players","contracts","start_roles","rules",
      "stripped", "stunned", "revealed", "vengeance"]:
      d[name] = self.__dict__[name]
    d = {
      "day": self.day,
      "phase": self.phase,
      "players": self.players,
      "contracts": self.contracts,
      "start_roles": self.start_roles,
      "rules" : self.rules,
      "stripped": list(self.stripped),
      "stunned": list(self.stunned),
      "revealed": list(self.revealed),
      "vengeance": self.vengeance,
    }
    return d

  # NOTE: User must reinit cast_main, cast_mafia, send_dm, halt_timer hooks!
  @staticmethod
  def from_json(d):
    mstate = MState(rules=d['rules'])
    mstate.day = d['day']
    mstate.phase = d['phase']
    mstate.players = OrderedDict(d['players'])
    mstate.contracts = d['contracts']
    mstate.start_roles = d['start_roles']
    mstate.rules = d['rules']
    mstate.stripped = set(d['stripped'])
    mstate.stunned = set(d['stunned'])
    mstate.revealed = set(d['revealed'])
    mstate.vengeance = d['vengeance']
    return mstate