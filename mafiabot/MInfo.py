
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
  "CHARGE_DIE_GUARD": "Oh no! Your charge [{charge}] has died, at the hands of [{aggressor}]!",
  "CHARGE_DIE_AGENT": "Congratulations! Your charge [{charge}] has died, at the hands of [{aggressor}]!",
  "SURVIVOR_DIE": "Oh no! You died at the hands of [{aggressor}]! As SURVIVOR, you have lost!",
  "CHARGE_ASSIGN":"Your charge is [{charge}]",
  "REFOCUS" : "You have been refocused as {new_role}",
  "DEATH":      "[{player}] was {role}",
  "STRIP":      "You were distracted...",
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
  "START":      "Start Game:\n",
  "WIN":   "{winning_team} won!",
  "IDIOT_WIN": "IDIOT [{idiot}] won! Everyone else lost!",
  "CONTRACT_WIN":"{role} [{player}] won! Charge: [{charge}]",
  "CONTRACT_LOSE":"{role} [{player}] lost! Charge: [{charge}]",
  "SHOW_ROLES": "Roles:\n{}",

  "INVALID_VOTER": "[{player_id}] cannot vote, not playing",
  "INVALID_VOTEE": "Cannot vote for [{player_id}], they are not playing",
  "INVALID_VOTE_PHASE": "Can only vote during Day",
  "INVALID_TARGET": "Invalid target: {text}",
  "INVALID_TARGETER": "You cannot target if you are not playing",
  "INVALID_TARGET_ROLE": "You cannot target if you don't have a targeting role",
  "INVALID_TARGETED" : "You cannot target [{target_id}] as they are not playing",
  "INVALID_TARGET_PHASE": "Can only target during Night",
  "INVALID_TARGET_STUNNED": "You are stunned. A stunned player can only select NOTARGET",
  "INVALID_ITARGET_PHASE": "Can only revenge target during Dusk",
  "INVALID_ITARGET_PLAYER": "You are not the one who needs vengeance",
  "INVALID_ITARGETED": "Could not target that player as they didn't vote for you",
  "MILK_SELF": "Ewwww please don't milk yourself...",
  "INVALID_MTARGET": "Invalid target: {text}",
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

ROLE_EXPLAIN= {
    "TOWN"  : ("The TOWN is a normal player in this game, the last "
               "line of defense against the mafia scum. They sniff out who the "
               "mafia are and convince their fellow town members to kill them "
               "during the day!"),
    "COP"   : ("The COP is the one of the most offensive members of "
               "the townspeople. During the Night, they send a direct message to MODERATOR "
               "with the letter of the person they want to investigate, and "
               "upon morning, MODERATOR will tell them whether that person is MAFIA or "
               "NOT MAFIA."),
    "DOCTOR": ("The DOCTOR's job is to save the townspeople from "
               "the mafia scum. During the Night, they send a direct message to MODERATOR"
               " with the letter of the person they want to save. If the mafia"
               " targets that person, they will have a near death experience, but "
               "survive."),
    "CELEB" : ("The CELEB is a celebrity. Everybody knows who they are, but everyone "
               "doesn't recognize them right now. CELEB can reveal themselves during Day "
               "by sending MODERATOR '/reveal' and then everyone will know they"
               " are Town. But they ought to be careful! "
               "They'll be quite the target once revealed!"),
    "MILLER" : ("The MILLER is pretty sus but they are actually on the side of Town... "
                "If the cop investigates them, they show up as MAFIA..."),
    "MILKY"  : ("The MILKY gives out some milk to someone every night. Other than "
                "that they are a normal townsperson. Don't milk yourself!"),
    "MAFIA" : ("The MAFIA is part of the mafia chat to talk "
               "privately with their co-conspirators. During the Day, they try not "
               "to get killed. During the Night, they choose somebody to kill!"),
    "GODFATHER" : ( "The GODFATHER is a leader of the mafia, up "
               "to no good! They use the mafia chat to conspire. If a cop "
               "investigates them, they'll see the GODFATHER as NOT MAFIA!"),
    "STRIPPER" : ("The STRIPPER is a member of the Mafia with a special ability. "
                  "During the night, they can distract one person. This person can't"
                  " do their job that night (and possibly the following day). "
                  "A distracted COP learns nothing, a distracted DOCTOR can't save,"
                  " and a distracted CELEB can't reveal for a full day!"),
    "GOON"  : ("D'oh! The GOON is a member of the Mafia that cannot help target "
               "another player in the mafia chat at night. You can notarget but "
               "you cannot target another player..."),
    "IDIOT" : ("The IDIOT's dream is to be such an"
               " annoyance that the townsfolk kill them in frustration. They don't"
               " care whether the mafia win or lose, as long as everyone"
               " votes for them."),
    "SURVIVOR" : ("The SURVIVOR's only goal is to survive until the end of the game. "
             "Help Mafia? Help Town? It's up to you but make sure you'll live!"),
    "GUARD" : ("The GUARD is tasked with protecting a charge. You win if that "
               "player survives until the end of the game."),
    "AGENT" : ("The AGENT is tasked with inviting the death of a charge. Whether "
               "by election or by directing murder, you win if your charge dies."),
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

# all of these could have an s tacked on?
general_help = {
  'role': "\n".join(ALL_ROLES),
  'rule': "\n".join(RULE_LIST),
}

def listMenu(players, notarget=True):
  ps = []
  c = 'A'
  for player in players:
    ps.append("{}: [{}]".format(c,player))
    c = chr(ord(c)+1)
  if notarget:
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

def dispStartRoles(start_roles):
  msg = ""
  for id,role in start_roles.items():
    msg += "[{}]: {}\n".format(id,role)
  return msg
