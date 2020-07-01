
from enum import Enum, auto

class MReqType(Enum):


  # Handled in MServer
  START = auto()
  WATCH = auto()
  HELP = auto()
  IN = auto()
  OUT = auto()
  RULE = auto()
  STATS = auto()
  FOCUS = auto()