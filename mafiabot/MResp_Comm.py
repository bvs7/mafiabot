from .MResp import MRespType, MResp, default_resp_lib
from .MRules import MRules
from .MPlayer import TARGETING_ROLES
from .MEx import NOTARGET

PUBLIC = [
  MRespType.VOTE_RETRACT,
  MRespType.VOTE,
  MRespType.REVEAL,
  MRespType.TIMER_DAY,
  MRespType.TIMER_NIGHT,
  MRespType.ELECT,
  MRespType.ELECT_NOKILL,
  MRespType.ELECT_IDIOT,
  MRespType.KILL,
  MRespType.MILK,
  MRespType.DAY_PREAMBLE,
  MRespType.DAY,
  MRespType.NIGHT,
  MRespType.DUSK,
  MRespType.IDIOT_KILL,
  MRespType.START,
  MRespType.TOWN_WIN,
  MRespType.MAFIA_WIN,
  MRespType.CONTRACT_WIN,
  MRespType.CONTRACT_LOSE,
  MRespType.DEATH,
  MRespType.UNKNOWN_REQ,
  MRespType.VOTE_ERROR,
  MRespType.MAIN_STATUS,
]

PRIVATE_MAFIA = [
  MRespType.MTARGET,
  MRespType.NIGHT,
  MRespType.NIGHT_OPTIONS,
  MRespType.UNKNOWN_REQ,
]

PRIVATE = [
  MRespType.TARGET,
  MRespType.NO_MILK_SELF,
  MRespType.REVEAL,
  MRespType.STRIP,
  MRespType.SAVE,
  MRespType.MILK,
  MRespType.INVESTIGATE,
  MRespType.START,
  MRespType.NIGHT_OPTIONS,
  MRespType.DUSK_OPTIONS,
  MRespType.CHARGE_REFOCUS,
  MRespType.CHARGE_REFOCUS_SELF,
  MRespType.SURVIVOR_IDIOT_DIE,
  MRespType.UNKNOWN_REQ,
]

ACT_LOOKUP ={
  "MAFIA":"kill",
  "STRIPPER":"strip",
  "DOCTOR":"save",
  "COP":"investigate",
  "MILKY":"milk",
}

class TestMComm: # Eventually extend MComm?

  def __init__(self, name, ids = {}):
    self.name = name
    self.ids = ids

  def cast(self, msg, notarget="None"):
    for i in self.ids:
      msg = msg.replace("[{}]".format(i), self.ids[i])
    msg = msg.replace("[{}]".format(NOTARGET), notarget)
    print("{} CAST: {}".format(self.name, msg))

  def send(self, msg, dest, notarget="None"):
    for i in self.ids:
      msg = msg.replace("[{}]".format(i), self.ids[i])
    msg = msg.replace("[{}]".format(NOTARGET), notarget)
    print("SEND {}: {}".format(dest,msg))

class MResp_Comm(MResp):

  def __init__(self, 
      main_chat=TestMComm("MAIN"), 
      mafia_chat=TestMComm("MAFIA"),
      mrules=MRules()):
    self.main = main_chat
    self.mafia = mafia_chat
    self.mrules = mrules
    # Assert that all RespTypes are implemented
    assert( all([rt in PUBLIC + PRIVATE_MAFIA + PRIVATE for rt in MRespType]) )

  def resp(self, typ : MRespType, **kwargs) -> None:

    # Split into categories
    if typ in PUBLIC:
      self.publicResponse(typ, **kwargs)
    if typ in PRIVATE_MAFIA:
      self.privateMafiaResponse(typ, **kwargs)
    if typ in PRIVATE:
      self.privateResponse(typ, **kwargs)

  def publicResponse(self, typ, **kwargs):
    
    known_roles = self.mrules["known_roles"]
    reveal_on_death = self.mrules["reveal_on_death"]

    msg = default_resp_lib[typ]

    if typ == MRespType.START:
      players = kwargs['players']
      msg += '\n'
      roleDict = self.makeRoleDict([p.role for p in players.values()])
      if known_roles == "ROLE":
        msg += self.dispRoleFromDict(roleDict)
      elif known_roles in ['TEAM','MAFIA']:
        msg += self.dispTeamFromDict(roleDict, known_roles)
      elif known_roles == 'OFF':
        msg += "Players: {}".format(len(players))
      else:
        AssertionError("known_roles bad value")
      thresh = int(len(players)/2) + 1
      msg += "\n{thresh} votes needed to elect".format(thresh=thresh)
      self.main.cast(msg)

    elif typ == MRespType.DEATH:
      if not reveal_on_death == "OFF":
        msg = "[{player}] was {role}"
        kwargs['role'] = self.dispRole(kwargs['role'], reveal_on_death)
        self.main.cast(msg.format(**kwargs))

    elif typ == MRespType.VOTE:
      msg += ", {}".format(self.dispVoteThresh(kwargs, former = False))
      if kwargs['former_votee'] != None:
        msg +=", ({})".format(self.dispVoteThresh(kwargs, former = True))
      self.main.cast(msg.format(**kwargs), notarget="NOKILL")

    elif typ == MRespType.VOTE_RETRACT:
      msg += ", ({})".format(self.dispVoteThresh(kwargs, former = True))
      self.main.cast(msg.format(**kwargs), notarget="NOKILL")
    
    elif typ == MRespType.KILL:
      know_if_saved = self.mrules['know_if_saved']
      target = kwargs['target']
      if not kwargs['success'] and not know_if_saved == 'OFF':
        if know_if_saved == "SAVED":
          pass #Use default
        elif know_if_saved == "SECRET":
          msg = "Someone was saved after being attacked by the mafia!"
      elif target == NOTARGET or (not kwargs['success'] and know_if_saved == 'OFF'):
        msg = "It seems nobody died last night..."
      else: # target is player and kill was sucessful
        pass # msg is already default
      self.main.cast(msg.format(**kwargs))
    
    elif typ == MRespType.MILK:
      blocked = kwargs['blocked']
      if not blocked:
        self.main.cast(msg.format(**kwargs))

    elif typ == MRespType.DAY:
      known_roles = self.mrules['known_roles']
      reveal_on_death = self.mrules['reveal_on_death']
      players = kwargs['players']
      msg += '\n'
      roleDict = self.makeRoleDict([p.role for p in players.values()])
      if known_roles == "ROLE" and reveal_on_death == "ROLE":
        msg += self.dispRoleFromDict(roleDict)
      elif known_roles in ['ROLE','TEAM'] and reveal_on_death in ['ROLE','TEAM']:
        msg += self.dispTeamFromDict(roleDict, "TEAM")
      elif known_roles in ['ROLE','TEAM', 'MAIFA'] and reveal_on_death in ['ROLE','TEAM','MAFIA']:
        msg += self.dispTeamFromDict(roleDict, 'MAFIA')
      elif known_roles == 'OFF' or reveal_on_death == 'OFF':
        msg += "Players: {}".format(len(players))
      else:
        NotImplementedError("known_roles or reveal_on_death unknown value: {} | {}".format(known_roles, reveal_on_death))
      thresh = int(len(players)/2) + 1
      msg += "\n{thresh} votes needed to elect".format(thresh=thresh)
      self.main.cast(msg)


    elif typ == MRespType.MAIN_STATUS:
      mstate = kwargs['mstate']
      msg = "Game #{game_id}, {phase} {day}".format(game_id=mstate.id, phase=mstate.phase, day=mstate.day)
      if mstate.phase == "Day":
        msg += ":\n" + self.dispVotes(mstate.players)
      elif mstate.phase == "Dusk":
        msg += ":\n[{}] is seeking revenge against :\n  [".format(
          mstate.venger) + "]\n  [".join(mstate.venges) + "]"
      known_roles = self.mrules['known_roles']
      reveal_on_death = self.mrules['reveal_on_death']
      players = mstate.players
      msg += '\n'
      roleDict = self.makeRoleDict([p.role for p in players.values()])
      if known_roles == "ROLE" and reveal_on_death == "ROLE":
        msg += self.dispRoleFromDict(roleDict)
      elif known_roles in ['ROLE','TEAM'] and reveal_on_death in ['ROLE','TEAM']:
        msg += self.dispTeamFromDict(roleDict, "TEAM")
      elif known_roles in ['ROLE','TEAM', 'MAIFA'] and reveal_on_death in ['ROLE','TEAM','MAFIA']:
        msg += self.dispTeamFromDict(roleDict, 'MAFIA')
      elif known_roles == 'OFF' or reveal_on_death == 'OFF':
        msg += "Players: {}".format(len(players))
      else:
        NotImplementedError("known_roles or reveal_on_death unknown value: {} | {}".format(known_roles, reveal_on_death))

      self.main.cast(msg, notarget="NOKILL")

    else:
      self.main.cast(msg.format(**kwargs))

  def privateMafiaResponse(self, typ, **kwargs):
    msg = default_resp_lib[typ]
    if typ == MRespType.NIGHT:
      msg = "Night falls. It's time to "+ACT_LOOKUP["MAFIA"]
      self.mafia.cast(msg)
    elif typ == MRespType.NIGHT_OPTIONS and kwargs['dest'] in ("ALL","MAFIA"):
      msg = msg.format(act=ACT_LOOKUP["MAFIA"])
      ps = self.listMenu(kwargs['players'])
      msg += "\n".join(ps)
      self.mafia.cast(msg, "No "+ACT_LOOKUP["MAFIA"])
    elif typ == MRespType.MTARGET:
      if kwargs['target'] == NOTARGET:
        msg = "You have decided not to act tonight"
      self.mafia.cast(msg.format(**kwargs))


  def privateResponse(self, typ, **kwargs):
    msg = default_resp_lib[typ]
    if typ == MRespType.NIGHT_OPTIONS:
      dest = kwargs['dest']
      players = kwargs['players']
      if dest == "ALL":
        dest = [p for p in players if players[p].role in TARGETING_ROLES]
      else:
        dest = [dest]
      if kwargs['stunned']:
        venges = kwargs['venges']
        for p in venges:
          if p in dest:
            dest.remove(p)
            s_msg = "You are stunned and cannot act tonight"
            self.main.send(s_msg,p)

      ps = self.listMenu(players)
      for d in dest:
        d_msg = msg.format(act=ACT_LOOKUP[players[d].role])
        d_msg += "\n".join(ps)
        self.main.send(d_msg,d,notarget="No "+ACT_LOOKUP[players[d].role])

    elif typ == MRespType.TARGET:
      dest = kwargs['actor']
      if kwargs['target'] == NOTARGET:
        msg = "You have decided not to act tonight"
      self.main.send(msg.format(**kwargs),dest)

    elif typ == MRespType.SAVE:
      know_if_saved_doc = self.mrules['know_if_saved_doc']
      know_if_saved_self = self.mrules['know_if_saved_self']
      if not kwargs['blocked'] and kwargs['useful']:
        if know_if_saved_doc:
          msg = "You successfully saved [{target}]!"
          self.main.send(msg.format(**kwargs),kwargs['actor'])
        if know_if_saved_self:
          msg = "You were saved!"
          self.main.send(msg,kwargs['target'])

    elif typ == MRespType.INVESTIGATE:
      blocked = kwargs['blocked']
      if not blocked:
        role = kwargs['role']
        if role == "GODFATHER":
          role = "TOWN"
        if role == "MILLER":
          role = "MAFIA"
        kwargs['role'] = self.dispRole(role, self.mrules['cop_strength'])
        self.main.send(msg.format(**kwargs), kwargs['target'])
      else:
        know_if_stripped = self.mrules['know_if_stripped']
        if know_if_stripped == "USEFUL":
          msg = default_resp_lib[MRespType.STRIP]
          self.main.send(msg, kwargs['actor'])

    elif typ == MRespType.MILK:
      if kwargs['blocked']:
        know_if_stripped = self.mrules['know_if_stripped']
        if know_if_stripped == "USEFUL":
          msg = default_resp_lib[MRespType.STRIP]
          self.main.send(msg, kwargs['actor'])

    elif typ == MRespType.STRIP:
      know_if_stripped = self.mrules['know_if_stripped']
      target = kwargs['target']
      role = kwargs['role']
      is_targeting = role in TARGETING_ROLES.union({"CELEB"})
      if know_if_stripped == "ON" or (know_if_stripped == "TARGET" and is_targeting):
        self.main.send(msg, target)

    elif typ == MRespType.NO_MILK_SELF:
      self.main.send(msg, kwargs['actor'])
    
    elif typ == MRespType.DUSK_OPTIONS:
      ps = self.listMenu(kwargs['venges'])
      msg += "\n".join(ps)
      self.main.send(msg, kwargs['idiot'])
    


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
  def dispVoteThresh(kwargs, former = False):
    former_str = "former_" if former else ""
    votee = former_str + "votee"
    if kwargs[votee] == NOTARGET:
      thresh = "{no_kill_thresh}"
      goal = 'for peace'
    else:
      thresh = "{thresh}"
      goal = 'to elect [{votee}]'.format(votee="{"+votee+"}")
    return "{votes}/{thresh} ".format(votes="{"+former_str+"votes}", thresh=thresh) + goal