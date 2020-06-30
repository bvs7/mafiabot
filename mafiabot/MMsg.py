# Msgs come from MState

from enum import Enum, auto


class MMsgType(Enum):
  VOTE_RETRACT = auto()
  VOTE = auto()
  MTARGET = auto()
  TARGET = auto()
  REVEAL = auto()
  TIMER_DAY = auto()
  TIMER_NIGHT = auto()
  ELECT = auto()
  ELECT_NOKILL = auto()
  ELECT_IDIOT = auto()
  KILL = auto()
  DEATH = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  NO_MILK_SELF = auto()
  INVESTIGATE = auto()
  DAY_PREAMBLE = auto()
  DAY = auto()
  NIGHT = auto()
  NIGHT_OPTIONS = auto()
  DUSK = auto()
  DUSK_OPTIONS = auto()
  IDIOT_KILL = auto()
  START = auto()
  TOWN_WIN = auto()
  MAFIA_WIN = auto()
  CHARGE_REFOCUS = auto()
  CHARGE_REFOCUS_SELF = auto()
  SURVIVOR_IDIOT_DIE = auto()
  CONTRACT_WIN = auto()
  CONTRACT_LOSE = auto()

default_msg_lib = {
  MMsgType.VOTE_RETRACT: "[{voter}] retracted vote for [{former_votee}]",
  MMsgType.VOTE:       "[{voter}] votes for [{votee}]",
  MMsgType.MTARGET:    "[{actor}] prepares to kill [{target}]",
  MMsgType.TARGET:     "You have targeted [{target}]",
  MMsgType.REVEAL:     "Reveal: [{actor}]",
  MMsgType.TIMER_DAY:  "Timer: nokill",
  MMsgType.TIMER_NIGHT:"Timer: some slept through the night",
  MMsgType.ELECT:      "[{target}] has been elected to be killed",
  MMsgType.ELECT_NOKILL:"You have elected not to kill anyone",
  MMsgType.ELECT_IDIOT: "... They were an IDIOT...",
  MMsgType.KILL:       "[{target}] was killed by the mafia!",
  MMsgType.DEATH:      "[{player}] was {role}",
  MMsgType.STRIP:      "You were distracted...",
  MMsgType.SAVE:       "[{target}] was saved after being attacked by the mafia!",
  MMsgType.MILK:       "[{target}] received milk in the night.",
  MMsgType.NO_MILK_SELF: "Ewww, please don't milk yourself in front of me",
  MMsgType.INVESTIGATE:"[{target}] is {role}",
  MMsgType.DAY_PREAMBLE:"Day dawns",
  MMsgType.DAY:        "Pick someone to elect.",
  MMsgType.NIGHT:      "Night falls",
  MMsgType.NIGHT_OPTIONS:"Pick someone to {act}:\n",
  MMsgType.DUSK:       "Oops! [{idiot}] was IDIOT. The sky darkens as their reddening eyes observe the crowd...",
  MMsgType.DUSK_OPTIONS: "Pick someone who voted for you to kill:\n",
  MMsgType.IDIOT_KILL: "[{actor}] kills [{target}] before the crowd can subdue them",
  MMsgType.START:      "Start Game:",
  MMsgType.TOWN_WIN:   "Town Wins",
  MMsgType.MAFIA_WIN:  "Mafia Wins",
  MMsgType.CHARGE_REFOCUS: "Refocus {role} [{player}] -> {new_role}, [{charge}] -> [{aggressor}]",
  MMsgType.CHARGE_REFOCUS_SELF: "Refocus {role} [{player}] -> {new_role}, [{target}] -> self",
  MMsgType.SURVIVOR_IDIOT_DIE: "{role} [{player}] died, killed by [{aggressor}]",
  MMsgType.CONTRACT_WIN:"{role} [{player}] won! Charge: [{charge}]",
  MMsgType.CONTRACT_LOSE:"{role} [{player}] lost! Charge: [{charge}]",