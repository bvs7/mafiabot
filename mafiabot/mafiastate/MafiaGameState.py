
from typing import List, Dict, Any, Optional, NewType
from enum import Enum

import json

from datetime import datetime

import logging

logging.basicConfig(level=logging.DEBUG)

PlayerID = NewType("PlayerID", int)

class MafiaGameEncodable:
    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__}")
        d = self.__dict__.copy()
        return {f"__{self.__class__.__name__}__":d}

    @classmethod
    def fromdict(cls, d):
        logging.debug(f"Decoding {cls}, {d}")
        m = cls()
        for key,value in d.items():
            m.__dict__[key] = value
        return m

class ChatHandle(MafiaGameEncodable):
    pass

class MafiaGameTime(MafiaGameEncodable, Enum):
    INIT = "INIT"
    DAWN = "DAWN"
    DAY = "DAY"
    DUSK = "DUSK"
    NIGHT = "NIGHT"
    EVENING = "EVENING"
    END = "END"

    def mafia_game_encode(self):
        logging.debug(f"Encoding MafiaGameTime: {self.value}")
        return {f"__{self.__class__.__name__}__" : self.value}

    @classmethod
    def fromdict(cls, str):
        return cls(str)        

class MafiaGamePhase(MafiaGameEncodable):
    
    def __init__(self):
        self.day : int = 0
        self.time : MafiaGameTime = MafiaGameTime.INIT
        self.start : datetime = None

    def __repr__(self):
        return f"{self.__class__.__name__}:{self.time.name} {self.day} @({self.start})"

class MafiaGamePhaseDay(MafiaGamePhase, MafiaGameEncodable):

    def __init__(self):
        super().__init__()
        self.time = MafiaGameTime.DAY
        self.votes : Dict[PlayerID,PlayerID] = {}

    def __repr__(self):
        return super().__repr__() + f" votes:{self.votes}"

class MafiaGamePhaseNight(MafiaGamePhase, MafiaGameEncodable):

    def __init__(self):
        super().__init__()
        self.time = MafiaGameTime.NIGHT
        self.targets : Dict[PlayerID,PlayerID] = {}
        self.mafia_target : Optional[PlayerID] = None
        
    def __repr__(self):
        return super().__repr__() + f" mafia_target: {self.mafia_target}, targets:{self.targets}"

class MafiaGameState(MafiaGameEncodable):

    def __init__(self):

        self.lobby_chat : Optional[ChatHandle] = None
        self.game_number : Optional[int] = 0

        self.main_chat : Optional[ChatHandle] = None
        self.mafia_chat : Optional[ChatHandle] = None

        self.phase : MafiaGamePhase = MafiaGamePhase()

        self.players : List[PlayerID] = []

        self.rules = None

    def save(self, fname):
        def default(o: Any) -> Any:
            if isinstance(o, MafiaGameEncodable):
                return o.mafia_game_encode()
                
            return json.JSONEncoder.default(o)
        
        logging.info(f"Saving game to {fname}: {self}")
        with open(fname, "w") as fp:
            s = json.dumps(self, default=default, indent=4)
            fp.write(s)
            return s

    @classmethod
    def load(cls, fname):
        decodables = dict([(f"__{cls.__name__}__",cls) for cls in MafiaGameEncodable.__subclasses__()])
        def object_hook(d):
            for class_name, cls in decodables.items():
                if class_name in d:
                    return cls.fromdict(d[class_name])
            return d
        
        with open(fname, "r") as fp:
            m = json.load(fp, object_hook=object_hook)
            return m

m = MafiaGameState()

m.phase = MafiaGamePhaseDay()

s = m.save("test.maf")

logging.debug("Intermediate str: " + s)

m2 = MafiaGameState.load("test.maf")

logging.debug(m2.__dict__)