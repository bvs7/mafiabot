from enum import Enum, auto
from typing import * # pylint: disable
from threading import Lock, Thread
import json
from collections import OrderedDict

from .MInfo import *
from .MRole import MRole, MTeam
from .MPlayer import MPlayer, MPlayerID, NOTARGET
from .MRules import MRules
from .util import VEnum
from .MRoleGen import MAssignment, MRoleGenType, MContract

__all__ = ['MState','MPhase','MVengeance','EndGameException','IdiotWinException','TeamWinException']

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
      msg = resp_lib["IDIOT_WIN"].format(idiot=idiot_id)
    super().__init__(msg)

class TeamWinException(EndGameException):
  def __init__(self, team, msg=None):
    self.team = team
    if msg == None:
      msg = resp_lib["WIN"].format(winning_team=team)
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

  def __init__(self, rules:MRules=MRules(), cast_main=print, cast_mafia=print, send_dm=print):

    self.cast_main = cast_main
    self.cast_mafia = cast_mafia
    self.send_dm = send_dm
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

    self.start_roles = "Init"

  def destroy(self):
    print("DEL MSTATE")

  def teststart(self, ids, roles, contracts):
    assignments = list(zip(ids,roles))
    self.start(assignments,contracts)

  # Is Iterable what we want? or would some other base class be better?
  def start(self, assignments:Iterable[MAssignment], 
                  contracts:Dict[MPlayerID, MContract]):
    # Check inputs for validity...
    assert isinstance(assignments, Iterable)
    assert len(assignments) >= 3
    assert isinstance(assignments[0][0],type(NOTARGET))
    assert isinstance(assignments[0][1], MRole) or isinstance(assignments[0][1],str)
    ids = [p_id for p_id,r in assignments]
    roles = [MRole(r) for p_id,r in assignments]
    assignments = list(zip(ids,roles))
    # Ensure starting roles are valid?
    maf = [m for m in roles if m.is_mafia()]
    n_players = len(roles)
    n_mafia = len(maf)
    assert not (n_mafia >= (n_players+1) // 2)
    assert not (n_mafia == 0)
    # Ensure starting contracts are valid
    for p_id, contract in contracts.items():
      assert p_id in ids
      assert roles[ids.index(p_id)] == contract.role
      assert contract.charge in ids
      assert contract.success == (contract.role in {MRole.SURVIVOR, MRole.GUARD})
    self.__start(assignments, contracts)

  def __start(self, assignments:Iterable[MAssignment],
                    contracts:Dict[MPlayerID, MContract]):
    # Do start things
    self.contracts = contracts
    for p_id,role in assignments:
      p = MPlayer(p_id, role)
      self.players[p_id] = p
      # Send role explain
      msg = p.role.expl()
      if p.role in {MRole.GUARD,MRole.AGENT}:
        msg += " " + resp_lib["CHARGE_ASSIGN"].format(charge=self.contracts[p_id].charge)
      
      for rule in MRules.relevant_rules[p.role]:
        sett = self.rules[rule]
        msg += "\n{}|{}: {}".format(rule,sett, MRules.RULE_BOOK[rule][sett])
      
      self.send_dm(msg, p_id)
    masons = set([p_id for p_id in self.players if self.players[p_id].role == MRole.MASON])
    
    for mason in masons:
      if len(masons) > 1:
        masons_str = ["[{}]".format(mason) for mason in (masons - {mason})]
        msg = resp_lib["MASON_REVEAL"].format("\n".join(masons_str))
        self.send_dm(msg, mason)
    self.start_roles = createStartRolesMsg(self.players,self.contracts)


    msg = resp_lib["START"]

    for p in self.players:
      msg += "\n" + ("[%s]"%p)
    known_roles = self.rules[MRules.known_roles]
    role_list = dispKnownRoles(makeRoleDict([r for p_id,r in assignments]), known_roles)
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
    main_msg = [""]
    if self.phase == MPhase.DAY:
      main_msg[0] += resp_lib["TIMER_DAY"] + "\n"
      self.__night(main_msg, nokill=True)
    elif self.phase == MPhase.NIGHT:
      main_msg[0] += resp_lib["TIMER_NIGHT"] + "\n"
      self.__dawn(main_msg)
    elif self.phase == MPhase.DUSK:
      main_msg[0] += resp_lib["TIMER_DUSK"] + "\n"
      self.__eliminate(self.vengeance.venges[-1], self.vengeance.idiot, main_msg)
      self.__night(main_msg, nokill=False)

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
          self.send_dm(resp_lib["STRIPPED"], reveal_id)
      else:
        self.revealed.add(reveal_id)
        msg = resp_lib['REVEAL'].format(actor=reveal_id, role=p.role)
        self.cast_main(msg)
    else:
      msg = resp_lib['REVEAL_REMINDER'].format(actor=reveal_id, role=p.role)
      self.cast_main(msg)

  @check_end
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
          voters = [p_id for (p_id,p) in self.players.items() if (
            p.vote == target_id and p_id != target_id)]
          self.vengeance = MVengeance(voters, actor_id, target_id)

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
              return self.__dusk(target_id, main_msg) # Go to dusk, don't kill idiot yet
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
        dm_msg = [""]
        if contract.role == "AGENT":
          dm_msg[0] += resp_lib["CHARGE_DIE_AGENT"].format(charge=target_id, aggressor=actor_id)
          contract.success = True
        elif contract.role =="GUARD":
          dm_msg[0] += resp_lib["CHARGE_DIE_GUARD"].format(charge=target_id, aggressor=actor_id)
          contract.success = False
        elif contract.role == "SURVIVOR":
          dm_msg[0] += resp_lib["SURVIVOR_DIE"].format(aggressor=actor_id)
          contract.success = False
        # Refocus if charge role is still alive
        if p in self.players and (
          (contract.role == "GUARD" and self.rules[MRules.charge_refocus_guard]) or
          (contract.role == "AGENT" and self.rules[MRules.charge_refocus_agent])):
          self.__refocus(p, target_id, actor_id, contract.role, dm_msg)
        if not dm_msg[0] == "":
          self.send_dm(dm_msg[0], p)

    n_players = len(self.players)
    n_mafia = len([p for p in self.players.values() if p.role.is_mafia()])
    if n_mafia == 0:
      self.__team_win(MTeam.Town, main_msg)
    elif n_mafia>= (n_players+1) // 2:
      self.__team_win(MTeam.Mafia, main_msg)

    role_msg = resp_lib["SHOW_ROLES"].format(self.start_roles)
    self.send_dm(role_msg, target_id)

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

    dm_msg[0] += "\n" + resp_lib["REFOCUS"].format(new_role=new_role)
    if new_role in (MRole.GUARD, MRole.AGENT):
       dm_msg[0] += "\n" + resp_lib["CHARGE_ASSIGN"].format(charge=new_charge)

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

  def __night(self, main_msg:List[str]=[""], nokill=False):
    self.__halt_timer()

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
    opts += '\n'.join(self.listMenu(self.players.keys()))
    for t_p in [p for p in self.players if self.players[p].role.is_targeting()]:
      msg = opts
      if t_p in self.stunned:
        msg = resp_lib["STUNNED"] + "\n" + opts
      self.send_dm(msg, t_p)
    self.cast_mafia(opts)

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
    self.send_dm(msg, actor_id)

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
      msg = resp_lib["INVALID_TARGETED"].format(target_id=target_id)
      raise InvalidActionException(msg)
    return self.__mtarget(targeter_id, target_id)

  def __mtarget(self, targeter_id : MPlayerID, target_id : Optional[MPlayerID]):
    self.mafia_target = (target_id, targeter_id)
    msg = resp_lib["MTARGET"].format(actor=targeter_id, target=target_id)
    self.cast_mafia(msg)
    if (self.mafia_target[0] != None) and all([p.target != None for p in self.players.values() if p.role.is_targeting()]):
      self.__dawn()
    return

  def __dawn(self, main_msg=[""]):
    self.__halt_timer()
    main_msg[0] += resp_lib['DAWN']

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
        target_id = stripper.target
        self.stripped.add(target_id)
        _know_if_stripped = self.rules[MRules.know_if_stripped]
        target = self.players[target_id]
        msg = resp_lib["STRIPPED"]
        if _know_if_stripped == "ON":
          self.send_dm(msg, target_id)
        elif _know_if_stripped == "TARGET":
          if target.role.is_targeting() or target.role == "CELEB":
            self.send_dm(msg, target_id)

  def __dawn_save(self) -> bool:
    target_saved = False
    if not self.mafia_target[0] in {NOTARGET, None}:
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
        try: # TODO: how to structure this better?
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
    self.__halt_timer()
    self.phase = MPhase.DAY
    self.mafia_target = (None, None)
    for p in self.players.values():
      p.target = None
    self.stunned = set()

  def __dusk(self, idiot_id, main_msg=[""]):
    self.__halt_timer()
    self.phase = MPhase.DUSK
    main_msg[0] += "\n" + resp_lib["DUSK"]
    self.cast_main(main_msg[0])
    opts = resp_lib["DUSK_OPTIONS"]
    opts += "\n".join(self.listMenu(self.vengeance.venges, notarget=False))
    self.send_dm(opts, self.vengeance.idiot)

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
    main_msg = [resp_lib['VENGEANCE'].format(actor=idiot_id, target=target_id)]
    try: # how to structure better?
      self.__eliminate(idiot_id, target_id, main_msg) # Eliminate target before idiot
      self.__eliminate(self.vengeance.final_vote, idiot_id, main_msg)
    except EndGameException as e:
      self.cast_main(main_msg[0])
      raise e
    self.cast_main(main_msg[0])
    self.__night()

  def __contract_result(self, contractor_id:MPlayerID, contract:MContract):
    if contract.success:
      msg = resp_lib["CONTRACT_WIN"].format(role=contract.role, player=contractor_id)
    else:
      msg = resp_lib["CONTRACT_LOSE"].format(role=contract.role, player=contractor_id, charge=contract.charge)
    if contract.role in {MRole.AGENT, MRole.GUARD}:
      msg += " " + resp_lib["CHARGE_REVEAL"].format(charge=contract.charge)
    return msg


  def __team_win(self, team, main_msg:List[str]):
    self.phase = MPhase.END
    self.halt_timer()
    msg = "\n" + resp_lib["WIN"].format(winning_team=team)
    for p_id,contract in self.contracts.items():
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += "\n" + resp_lib["SHOW_ROLES"].format(self.start_roles)
    main_msg[0] += msg
    raise TeamWinException(team, msg)

  def __idiot_win(self, idiot, main_msg:List[str]):
    self.phase = MPhase.END
    self.halt_timer()
    msg = "\n" + resp_lib["IDIOT_WIN"].format(idiot)
    for p_id,contract in self.contracts.items():
      msg += "\n" + self.__contract_result(p_id, contract)

    msg += resp_lib["SHOW_ROLES"].format(self.start_roles)
    main_msg[0] += msg
    raise IdiotWinException(idiot, msg)

  def main_status(self):
    return "TODO" # TODO

  def mafia_status(self):
    return "TODO" # TODO

  def dm_status(self, player_id):
    return "TODO" # TODO

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
    for name in ["day","phase","players","contracts","start_roles","rules"]:
      d[name] = self.__dict__[name]
    return d

  # NOTE: User must reinit cast_main, cast_mafia, send_dm, halt_timer hooks!
  @staticmethod
  def from_json(d):
    mstate = MState(d['rules'])
    mstate.day = d['day']
    mstate.phase = d['phase']
    mstate.players = d['players']
    mstate.contracts = d['contracts']
    mstate.start_roles = d['start_roles']
    return mstate