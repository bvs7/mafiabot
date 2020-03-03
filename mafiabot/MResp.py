
from typing import Dict
from enum import Enum, auto

# Resp must implement all of these Response types?
class MRespType(Enum):
  VOTE_RETRACT = auto()
  VOTE_NOKILL = auto()
  VOTE_PLAYER = auto()
  MTARGET = auto()
  TARGET = auto()
  REVEAL = auto()
  TIMER_DAY = auto()
  TIMER_NIGHT = auto()
  ELECT = auto()
  KILL = auto()
  STRIP = auto()
  SAVE = auto()
  MILK = auto()
  INVESTIGATE = auto()
  DAY = auto()
  NIGHT = auto()
  START = auto()
  TOWN_WIN = auto()
  MAFIA_WIN = auto()


debug_resp_lib = {
  MRespType.VOTE_RETRACT: "Vote Retract: {voter}",
  MRespType.VOTE_NOKILL: "Vote nokill: {voter}, {remain} more for peace.",
  MRespType.VOTE_PLAYER: "Vote: {voter} -> {votee}, {remain} more to elect.",
  MRespType.MTARGET:    "Mafia Target: {target}",
  MRespType.TARGET:     "Target: {player} -> {target}",
  MRespType.REVEAL:     "Reveal: {player}",
  MRespType.TIMER_DAY:  "Timer: nokill",
  MRespType.TIMER_NIGHT:"Timer: some slept through the night",
  MRespType.ELECT:      "Elect: {electee}",
  MRespType.KILL:       "Kill: {target}",
  MRespType.STRIP:      "Strip: {actor} -> {target}",
  MRespType.SAVE:       "Save: {actor} -> {target}",
  MRespType.MILK:       "Milk: {actor} -> {target}",
  MRespType.INVESTIGATE:"Investigate: {actor} -> {target}",
  MRespType.DAY:        "Day",
  MRespType.NIGHT:      "Night",
  MRespType.START:      "Start Game:\n{players}",
  MRespType.TOWN_WIN:   "Town Wins",
  MRespType.MAFIA_WIN:  "Mafia Wins",
}

class MResp:

  def resp(self, typ : MRespType, **kwargs) -> bool:
    # This function should be overloaded by a subclass?

    try:
      print(debug_resp_lib[typ].format(**kwargs))
    except Exception as e:
      print(e)
      raise(e)

    return True
