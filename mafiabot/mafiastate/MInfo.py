
from .MRole import MRole, MTeam
from ..util import VEnum, auto
from ..resp_lib import resp_lib

def teamFromRole(role):
  if role.is_town():
    return "Town"
  if role.is_mafia():
    return "Mafia"
  if role.is_rogue():
    return "Rogue"

def dispRole(role, level="ON"):
  if level in ["ON","ROLE"]:
    return role
  elif level == "TEAM":
    m = teamFromRole(role)
    return m + " Aligned"
  elif level == "MAFIA":
    m = "Mafia" if teamFromRole(role)=="Mafia" else "Not Mafia"
    return m + " Aligned"
  else:
    return "[REDACTED]"

def makeRoleDict(roles):
  roleDict = {}
  for role in roles:
    if not role in roleDict:
      roleDict[role] = 0
    roleDict[role] += 1
  return roleDict


def dispRoleFromDict(roleDict):
  msgs = []
  for role in MRole.__members__.values():
    if role in roleDict:
      msgs.append("{role}: {amt}".format(role=role, amt=roleDict[role]))
  return '\n'.join(msgs)


def dispTeamFromDict(roleDict, known_roles):
  Town = 0
  Mafia = 0
  Rogue = 0
  for role,n in roleDict.items():
    if role.is_town():
      Town += n
    elif role.is_mafia():
      Mafia += n
    elif role.is_rogue():
      Rogue += n
  if known_roles == "TEAM":
    if Rogue > 0:
      return "Town Aligned: {}\nMafia Aligned: {}\nRogue: {}\nTotal: {}".format(Town, Mafia, Rogue, Town+Mafia+Rogue)
    else:
      return "Town Aligned: {}\nMafia Aligned: {}\nTotal: {}".format(Town,Mafia, Town+Mafia)
  elif known_roles == "MAFIA":
    return "Mafia Aligned: {}\nTotal: {}".format(Mafia, Town+Mafia+Rogue)
  else:
    raise ValueError(str(known_roles) + " wasn't TEAM or MAFIA")

def dispKnownRoles(roles, known_roles):
  roleDict = makeRoleDict(roles)
  if known_roles == "ROLE":
    return dispRoleFromDict(roleDict)
  elif known_roles in ("TEAM", "MAFIA"):
    return dispTeamFromDict(roleDict, known_roles)
  elif known_roles == "OFF":
    return "Players: {}".format(len(roleDict))

def createStartRolesMsg(players,contracts):
  msg = ""
  for p in players.values():
    msg += "\n[{}]: {}".format(p.id, p.role)
    if p.role in {"GUARD", "AGENT"}:
      msg += "([{}])".format(contracts[p.id].charge)
  return msg

def getNewGameID():
  try:
    f = open("./data/game_id", 'r')
    i = int(f.read().strip())
    f.close()
    f = open("./data/game_id", 'w')
    f.write(str(i+1))
    f.close()
  except Exception as e:
    print("Failed to make game id: {}".format(e))
    return -1
  return i
      