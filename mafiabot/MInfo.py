
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

  @staticmethod
  def parseCmd(s):
    m = MCmd.__members__
    for k,v in m:
      if v == s:
        return k
    return None
      

ACCESS_KW = "/"

VOTE_CMD = "vote"
TARGET_CMD = "target"
REVEAL_CMD = "reveal"
TIMER_CMD = "timer"
UNTIMER_CMD = "untimer"
HELP_CMD = "help"
STATUS_CMD = "status"
START_CMD = "start"
IN_CMD = "in"
OUT_CMD = "out"
RULE_CMD = "rule"
WATCH_CMD = "watch"


GAME_MAIN_COMMANDS = [
  VOTE_CMD,
  STATUS_CMD,
  HELP_CMD,
  TIMER_CMD,
  UNTIMER_CMD,
  RULE_CMD,
]

GAME_MAFIA_COMMANDS = [
  TARGET_CMD,
  STATUS_CMD,
  HELP_CMD,
  RULE_CMD,
]

GAME_DM_COMMANDS = [
  TARGET_CMD,
  REVEAL_CMD,
  STATUS_CMD,
  HELP_CMD,
  RULE_CMD,
]

LOBBY_COMMANDS = [
  START_CMD,
  IN_CMD,
  OUT_CMD,
  RULE_CMD,
  STATUS_CMD,
  HELP_CMD,
  WATCH_CMD,
  RULE_CMD,
]

main_command_help = {
  VOTE_CMD: ('/vote [target]\n  [target] can be a @mention of another player,'
    ' "me" to vote for yourself, "nokill" to vote to pass to Night peacefully,'
    ' or something else to retract your vote.\n  Voting happens during the Day'
    ' and ends with electing someone to kill. This is the main way Town kills Mafia.'),
  STATUS_CMD: ('/status\n  Display info about the state of the game, such as'
    ' who is playing, voting status, timer status, etc.'),
  RULE_CMD: ('/rule [rule]\n If [rule] is blank, display the current rule set.'
    ' otherwise display the possible settings for [rule] if such a rule exists'),
  HELP_CMD: ('/help [topic]\n Get help on a topic. [topic] can be "command",'
    ' a rule, a ROLE, a Team, or some other topics (try /help index to list them)'),
}

mafia_command_help = {
  TARGET_CMD: ('/target [target]\n  [target] is the letter of the player you'
    ' want to kill (try /status to see letters). A GOON can\'t target'),
  STATUS_CMD: ('/status\n  Display info about the state of the game, such as'
    ' who is playing, voting status, timer status, etc.'),
  RULE_CMD: ('/rule [rule]\n If [rule] is blank, display the current rule set.'
    ' otherwise display the possible settings for [rule] if such a rule exists'),
  HELP_CMD: ('/help [topic]\n Get help on a topic. [topic] can be "command",'
    ' a rule, a ROLE, a Team, or some other topics (try /help index to list them)'),
}

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
  "INVALID_ITARGET":  "Invalid target: {text}",
  "INVALID_ITARGETED": "Could not target that player as they didn't vote for you",
  "INVALID_TARGET_MILK_SELF": "Ewww... Please don't milk yourself in front of me...",
  "INVALID_MTARGET": "Invalid target: {text}",
  "INVALID_MTARGET_PLAYER": "You cannot target if you are not playing or do not have a mafia role",
  "INVALID_MTARGET_PHASE": "Can only target during Night",
  "INVALID_REVEAL_PLAYER": "You cannot reveal if you are not playing",
  "INVALID_REVEAL_ROLE": "You cannot reveal if you are not a CELEB",
  "INVALID_REVEAL_PHASE": "Can only reveal during Day",
}


RULE_LIST = [
  "known_roles",
  "reveal_on_death",
  "start_night",
  "know_if_saved",
  "know_if_saved_doc",
  "know_if_saved_self",
  "idiot_vengeance",
  "charge_refocus_guard",
  "charge_refocus_agent",
  "know_if_stripped",
  "no_milk_self",
  "cop_strength",
  "unique_night_act"
]

def listMenu(players, notarget=True):
  p_ids = list(players.keys())
  if notarget:
    p_ids.append("NOTARGET")
  p_lists = []
  while len(p_ids) > 0:
    l = min(len(p_ids),26)
    p_lists.append(p_ids[:l])
    p_ids = p_ids[l:]
  print(p_lists)
  ps = []
  for i,p_list in enumerate(p_lists):
    prefix = "" if i==0 else chr(ord('A')+i-1)
    c = 'A'
    for p_id in p_list:
      ps.append("{}{}: [{}]".format(prefix,c,p_id))
      c = chr(ord(c)+1)
  return ps

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

def getStateID():
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
      