
from .MRole import MRole, MTeam
from .util import VEnum, auto

class MCmd(VEnum):
  VOTE = "vote"
  TARGET = "target"
  REVEAL = "reveal"
  TIMER = "timer"
  UNTIMER = "untimer"
  HELP = "help"
  STATUS = "status"
  START = "start"
  IN = "in"
  OUT = "out"
  RULE = "rule"
  WATCH = "watch"
  FOCUS = "focus"
  END = "end"

  @staticmethod
  def parseCmd(s):
    m = MCmd.__members__
    for k,v in m:
      if v == s:
        return k
    return None

  def is_main(self):
    return self in {
      MCmd.VOTE,
      MCmd.TIMER,
      MCmd.UNTIMER,
      MCmd.STATUS,
      MCmd.HELP,
      MCmd.END
    }

  def is_mafia(self):
    return self in {
      MCmd.TARGET,
      MCmd.STATUS,
      MCmd.HELP,
    }
  
  def is_game_dm(self):
    return self in {
      MCmd.TARGET,
      MCmd.REVEAL,
      MCmd.STATUS,
      MCmd.HELP
    }

  def is_lobby(self):
    return self in {
      MCmd.START,
      MCmd.IN,
      MCmd.OUT,
      MCmd.WATCH,
      MCmd.STATUS,
      MCmd.HELP,
    }
      

ACCESS_KW = "/"

resp_lib = {
  "VOTE_CHANGE": "[{voter}] retracts vote for [{f_votee}] and votes for [{votee}]",
  "VOTE_RETRACT": "[{voter}] retracted vote for [{f_votee}]",
  "VOTE":       "[{voter}] votes for [{votee}]",
  "VOTE_UPDATE": ", {n_voters}/{thresh} to elect [{votee}]",
  "VOTE_UPDATE_NOKILL": ", {n_voters}/{nokill_thresh} for peace",
  "MTARGET":    "[{actor}] prepares to kill [{target}]",
  "TARGET":     "You have targeted [{target}]",
  "NOTARGET":   "You have decided not to target anyone tonight",
  "REVEAL":     "[{actor}] is {role}",
  "REVEAL_REMINDER": "Remember, [{actor}] is {role}",
  "TIMER_DAY":  "Timer: nokill",
  "TIMER_NIGHT":"Timer: some slept through the night",
  "ELECT":      "[{target}] has been elected to be killed",
  "ELECT_NOKILL":"You have elected not to kill anyone",
  "ELECT_IDIOT": "... They were an IDIOT...",
  "ELECT_DAY"  :" The Day will continue!",
  "ELECT_STUN" :" All who voted for the IDIOT will be stunned tonight.",
  "KILL":       "[{target}] was killed by the mafia!",
  "KILL_FAIL_QUIET":  "It seems nobody died last night...",
  "VENGEANCE":  "[{actor}] takes [{target}] with them",
  "ELIMINATE" : "[{target}] was {role}",
  "ELIMINATE_ANON":"[{target}] has died",
  "CHARGE_DIE_GUARD": "Oh no! Your charge, [{charge}], has died at the hands of [{aggressor}]!",
  "CHARGE_DIE_AGENT": "Congratulations! Your charge, [{charge}], has died at the hands of [{aggressor}]!",
  "SURVIVOR_DIE": "Oh no! You died at the hands of [{aggressor}]! As SURVIVOR, you have lost!",
  "REFOCUS" : "You have been refocused to {new_role}",
  "CHARGE_ASSIGN":"Your charge is [{charge}]",
  "MASON_REVEAL": "Your fellow MASONs are:\n{}",
  "STRIPPED":   "You were distracted...",
  "STUN":       "You are stunned until next morning",
  "STUNNED":    "While stunned you can only target NOTARGET",
  "SAVE":       "[{target}] was saved after being attacked by the mafia!",
  "SAVE_SECRET":"Somebody was saved after being attacked by the mafia!",
  "SAVE_DOC":   "You saved your patient!",
  "SAVE_SELF":  "You were saved!",
  "MILK":       "[{target}] received milk in the night.",
  "NO_MILK_SELF": "Ewww, please don't milk yourself in front of me",
  "INVESTIGATE":"[{target}] is {role}",
  "DAWN":       "Day dawns",
  "DAY":        "Pick someone to elect.",
  "NIGHT":      "Night falls",
  "NIGHT_OPTIONS":"Pick someone to target:\n",
  "DUSK":       "The sky darkens as their reddening eyes observe the crowd...",
  "DUSK_OPTIONS": "Pick someone who voted for you to kill:\n",
  "START":      "Start Game. Everyone is in the MAIN CHAT:",
  "START_MAFIA": "Start Game. You are the Mafia!",
  "WIN":   "{winning_team} won!",
  "IDIOT_WIN": "IDIOT [{idiot}] won!",
  "CONTRACT_WIN":"{role} [{player}] won!",
  "CONTRACT_LOSE":"{role} [{player}] lost!",
  "CHARGE_REVEAL":"Charge: [{charge}]",
  "SHOW_ROLES": "Roles:{}",

  "INVALID_VOTER": "[{player_id}] cannot vote, not playing",
  "INVALID_VOTEE": "Cannot vote for [{player_id}], they are not playing",
  "INVALID_VOTE_PHASE": "Can only vote during Day",
  "INVALID_TARGET": "Invalid target: {text}",
  "INVALID_TARGETER": "You cannot target if you are not playing",
  "INVALID_TARGET_ROLE": "You cannot target if you don't have a targeting role",
  "INVALID_TARGETED" : "You cannot target [{target_id}] as they are not playing",
  "INVALID_TARGET_PHASE": "Can only target during Night",
  "INVALID_TARGET_STUNNED": "You are stunned. A stunned player can only select NOTARGET",
  "INVALID_ITARGET_PHASE": "Can only take revenge during Dusk",
  "INVALID_ITARGET_PLAYER": "You are not the one who needs vengeance",
  "INVALID_ITARGETED": "Could not target that player as they didn't vote for you",
  "INVALID_TARGET_MILK_SELF": "Ewww... Please don't milk yourself in front of me...",
  "INVALID_MTARGET": "Invalid target: {text}",
  "INVALID_MTARGET_PLAYER": "You cannot target if you are not playing or do not have a mafia role",
  "INVALID_MTARGET_PHASE": "Can only target during Night",
  "INVALID_REVEAL_PLAYER": "You cannot reveal if you are not playing",
  "INVALID_REVEAL_ROLE": "You cannot reveal if you are not a CELEB",
  "INVALID_REVEAL_PHASE": "Can only reveal during Day",
  "INVALID_ACTION_END": "Game has ended, gg no re",
}


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
    f = open("../data/game_id", 'r')
    i = int(f.read().strip())
    f.close()
    f = open("../data/game_id", 'w')
    f.write(str(id+1))
    f.close()
  except Exception as e:
    print("Failed to make game id: {}".format(e))
    return -1
  return i
      