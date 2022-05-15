
from datetime import datetime
from typing import List, Any, Dict
from json import JSONEncoder

class MPlayer:
    def __init__(self, p_id):
        self.p_id = p_id
    
    def __hash__(self):
        return hash(self.p_id)

    def __eq__(self, o):
        if isinstance(o, MPlayer):
            return self.p_id == o.p_id
        return self.p_id == o

class MRules:
    pass

class MPhase:

    def __init__(self, day:int, phase:str, start:datetime):
        self.day = day
        self.phase = phase
        self.start = start

class DayPhase(MPhase):

    def __init__(self, day:int, phase:str, start:datetime,
                 players:List[MPlayer]):
        super().__init__(day,phase,start)
        self.votes : Dict[MPlayer, MPlayer] = dict.fromkeys(players, None)

class NightPhase(MPhase):
    
    def __init__(self, day:int, phase:str, start:datetime,
                 players:List[MPlayer]):
        super().__init__(day,phase,start)
        self.targets = dict.fromkeys(
            [p for p in players if isinstance(p.role, TargetingRole)], 
            None)

class MState:

    def __init__(self):
        self.lobby_id : int = None # TODO
        self.game_number : int = None # TODO
        self.main_id : int = None # TODO
        self.mafia_id : int = None # TODO
        self.phase : MPhase = None # TODO
        self.players : List[MPlayer] = None # TODO
        self.rules : MRules = None # TODO

    # TODO: save and restore

    @staticmethod
    def restore(fname:str):
        return MState()
    
    def save(self, fname:str):
        


class MStateEncoder(JSONEncoder):

    def default(self, o: Any) -> Any:
        if isinstance(o, MState):
            return {"__MState__":o.__dict__.copy()}
        if isinstance(o, MPhase):
            d = {
                "day":o.day,
                "phase":o.phase,
                "start":str(o.start),
            }
            if isinstance(o, DayPhase):
                
                d["votes"] = 

        return super().default(o)