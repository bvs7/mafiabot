from .MResp import MRespType, MResp, default_resp_lib
from .MRules import MRules

PUBLIC = [
  MRespType.VOTE_RETRACT,
  MRespType.VOTE_NOKILL,
  MRespType.VOTE_PLAYER,
  MRespType.REVEAL,
  MRespType.TIMER_DAY,
  MRespType.TIMER_NIGHT,
  MRespType.ELECT,
  MRespType.KILL,
  MRespType.SAVE,
  MRespType.MILK,
  MRespType.DAY_PREAMBLE,
  MRespType.DAY,
  MRespType.NIGHT,
  MRespType.START,
  MRespType.TOWN_WIN,
  MRespType.MAFIA_WIN,
  MRespType.CONTRACT_WIN,
  MRespType.CONTRACT_LOSE,
]

PRIVATE_MAFIA = [
  MRespType.MTARGET,
]

PRIVATE = [
  MRespType.TARGET,
  MRespType.REVEAL,
  MRespType.STRIP,
  MRespType.SAVE,
  MRespType.MILK,
  MRespType.INVESTIGATE,
  MRespType.START,
  MRespType.CHARGE_REFOCUS,
  MRespType.CHARGE_REFOCUS_SELF,
  MRespType.SURVIVOR_IDIOT_DIE,
]

class TestMComm: # Eventually extend MComm?

  def __init__(self, name, ids = {}):
    self.name = name
    self.ids = ids

  def cast(self, msg):
    for i in self.ids:
      msg = msg.replace("[{}]".format(i), self.ids[i])
    print("{} CAST: {}".format(self.name, msg))

class MResp_GroupMe(MResp):

  def __init__(self, 
      main_chat=TestMComm("MAIN"), 
      mafia_chat=TestMComm("MAFIA"),
      rules=MRules()):
    self.main = main_chat
    self.mafia = mafia_chat
    self.rules = rules
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

    if typ == MRespType.START:
      players = kwargs['players']
      msg = default_resp_lib[typ] + '\n'
      roleDict = self.makeRoleDict([p.role for p in players.values()])
      known_roles = self.rules["known_roles"]
      if known_roles == "ROLE":
        msg += self.dispRoleFromDict(roleDict)
      elif known_roles in ['TEAM','MAFIA']:
        msg += self.dispTeamFromDict(roleDict, known_roles)
      elif known_roles == 'OFF':
        msg += "Players: {}".format(len(players))
      else:
        AssertionError("known_roles bad value")
      self.main.cast(msg)

    else:
      self.main.cast(default_resp_lib[typ].format(**kwargs))

  def privateMafiaResponse(self, typ, **kwargs):
    pass

  def privateResponse(self, typ, **kwargs):
    pass