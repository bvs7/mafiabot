known_roles = "known_roles"
reveal_on_death = "reveal_on_death"
know_if_saved = "know_if_saved"
know_if_saved_doc = "know_if_saved_doc"
know_if_saved_self = "know_if_saved_self"
start_night = "start_night"
charge_refocus_guard = "charge_refocus_guard"
charge_refocus_agent = "charge_refocus_agent"
idiot_vengeance = "idiot_vengeance"
know_if_stripped = "know_if_stripped"
no_milk_self = "no_milk_self"
cop_strength = "cop_strength"

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

GAME_MAIN_COMMANDS = [
  VOTE_CMD,
  STATUS_CMD,
  HELP_CMD,
  TIMER_CMD,
  UNTIMER_CMD,
]

GAME_MAFIA_COMMANDS = [
  TARGET_CMD,
  STATUS_CMD,
  HELP_CMD,
]

GAME_DM_COMMANDS = [
  TARGET_CMD,
  REVEAL_CMD,
  STATUS_CMD,
  HELP_CMD,
]

LOBBY_COMMANDS = [
  START_CMD,
  IN_CMD,
  OUT_CMD,
  RULE_CMD,
  STATUS_CMD,
  HELP_CMD,
]

default_resp_lib = {
  "VOTE_RETRACT": "[{voter}] retracted vote for [{f_votee}]",
  "VOTE":       "[{voter}] votes for [{votee}]",
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
  "CHARGE_DIE": "[{target}] has died",
  "CHARGE_KILLED": ", at the hands of [{aggressor}]",
  "DEATH":      "[{player}] was {role}",
  "STRIP":      "You were distracted...",
  "STUN":       "You are stunned until next morning",
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
  "START":      "Start Game:\n",
  "WIN":   "{winning_team} Wins!",
  "REFOCUS": "Refocus {role} [{actor}] -> {new_role}, [{target}] -> [{aggressor}]",
  "REFOCUS_SELF": "Refocus {role} [{actor}] -> {new_role}, [{target}] -> self",
  "SURVIVOR_IDIOT_DIE": "{role} [{player}] died, killed by [{aggressor}]",
  "CONTRACT_WIN":"{role} [{player}] won! Charge: [{charge}]",
  "CONTRACT_LOSE":"{role} [{player}] lost! Charge: [{charge}]",
  "CONTRACT_RESULT":"{role} [{contractor}] {result}! Charge: [{charge}]",

  "INVALID_VOTE_PLAYER": "[{player_id}] cannot vote, not playing",
  "INVALID_VOTE_PHASE": "Can only vote during Day",
  "INVALID_TARGET": "Invalid target: {target_letter}",
  "INVALID_TARGET_PLAYER": "You cannot target if you are not playing or do not have a targeting role",
  "INVALID_TARGET_PHASE": "Can only target during Night",
  "MILK_SELF": "Ewwww please don't milk yourself...",
  "INVALID_MTARGET": "Invalid target: {target_letter}",
  "INVALID_MTARGET_GOON": "A GOON can only choose no kill",
  "INVALID_MTARGET_PLAYER": "You cannot target if you are not playing or do not have a mafia role",
  "INVALID_MTARGET_PHASE": "Can only target during Night",
  "INVALID_REVEAL_PLAYER": "You cannot reveal if you are not playing or you are not a CELEB",
  "INVALID_REVEAL_PHASE": "Can only reveal during Day",
}

TOWN_ROLES = [
  'TOWN',
  'COP',
  'DOCTOR',
  'CELEB',
  'MILLER',
  'MILKY',
  'MASON',
]

MAFIA_ROLES = [
  'MAFIA',
  'GODFATHER',
  'STRIPPER',
  'GOON',
]

ROGUE_ROLES = [
  'IDIOT',
  'SURVIVOR',
  'GUARD',
  'AGENT',
]

ALL_ROLES = TOWN_ROLES + MAFIA_ROLES + ROGUE_ROLES

TARGETING_ROLES = {
  'COP',
  'DOCTOR',
  'MILKY',
  'STRIPPER',
}

CONTRACT_ROLES = {
  'IDIOT',
  'SURVIVOR',
  'GUARD',
  'AGENT',
}

def listMenu(players):
  ps = []
  c = 'A'
  for player in players:
    ps.append("{}: [{}]".format(c,player))
    c = chr(ord(c)+1)
  ps.append("{}: [NOTARGET]".format(c))
  return ps

def teamFromRole(role):
  if role in TOWN_ROLES:
    return "Town"
  if role in MAFIA_ROLES:
    return "Mafia"
  if role in ROGUE_ROLES:
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
  for role in ALL_ROLES:
    if role in roleDict:
      msgs.append("{role}: {amt}".format(role=role, amt=roleDict[role]))
  return '\n'.join(msgs)


def dispTeamFromDict(roleDict, known_roles):
  Town = 0
  Mafia = 0
  Rogue = 0
  for role,n in roleDict.items():
    if role in TOWN_ROLES:
      Town += n
    elif role in MAFIA_ROLES:
      Mafia += n
    elif role in ROGUE_ROLES:
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

def dispKnownRoles(roleDict, known_roles):
  if known_roles == "ROLE":
    return dispRoleFromDict(roleDict)
  elif known_roles in ("TEAM", "MAFIA"):
    return dispTeamFromDict(roleDict, known_roles)
  elif known_roles == "OFF":
    return "Players: {}".format(len(roleDict))


def getGames(MChatType):
  return []

def getDMs(MDMType):
  return MDMType()
