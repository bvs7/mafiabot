
from typing import List, Dict, Any, Optional, NewType, Set
from collections import namedtuple
from enum import Enum, auto

import json

from datetime import datetime

DATETIME_DEFAULT_FMT = "%Y-%m-%d %H:%M:%S.%f"

import logging

logging.basicConfig(level=logging.DEBUG)

PlayerID = NewType("PlayerID", int)
NOTARGET = PlayerID(0)

class MafiaGameEncodable:
    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        d = self.__dict__.copy()
        return {f"__{self.__class__.__name__}__":d}

    @classmethod
    def fromdict(cls, d):
        m = cls()
        for key,value in d.items():
            m.__dict__[key] = value
        return m


class Team(MafiaGameEncodable, Enum):
    Town = "Town"
    Mafia = "Mafia"
    Rogue = "Rogue"

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        return {f"__{self.__class__.__name__}__" : self.value}

    @classmethod
    def fromdict(cls, str):
        return cls(str) 

# Might also be called player? Base class for anyone playing
class Role(MafiaGameEncodable, Enum):
    TOWN = "TOWN", Team.Town
    COP = "COP", Team.Town, "targeting"
    DOCTOR = "DOCTOR", Team.Town, "targeting"
    CELEB = "CELEB", Team.Town
    MILLER = "MILLER", Team.Town
    MILKY = "MILKY", Team.Town, "targeting"

    MAFIA = "MAFIA", Team.Mafia
    GODFATHER = "GODFATHER", Team.Mafia
    STRIPPER = "STRIPPER", Team.Mafia, "targeting"
    GOON = "GOON", Team.Mafia

    IDIOT = "IDIOT", Team.Rogue
    SURVIVOR = "SURVIVOR", Team.Rogue
    GUARD = "GUARD", Team.Rogue, "contracting"
    AGENT = "AGENT", Team.Rogue, "contracting"

    def __new__(cls, value, team, special=""):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.team = team
            
        obj.targeting = "targeting" in special
        obj.contracting = "contracting" in special
        return obj

    def __repr__(self):
        return f"{self.name}"

    def __lt__(self, other):
        if not isinstance(other, Role):
            raise ValueError
        k = list(Role.__dict__.keys())
        return k.index(self.name) < k.index(other.name)

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        return {f"__{self.__class__.__name__}__" : self.value}

    @classmethod
    def fromdict(cls, str):
        return cls(str)  

class Player(MafiaGameEncodable):
    def __new__(cls, id:PlayerID, role:Role, *args, **kwargs):
        if role.contracting:
            cp = object.__new__(ContractingPlayer)
            return cp
        else:
            p = object.__new__(cls)
            p.__init__(id,role,*args,**kwargs)
            return p
    
    def __init__(self, id:PlayerID, role:Role):
        self.id = id
        self.role = role
    
    def __repr__(self):
        return f"<{self.__class__.__name__}:{self.id},{self.role.name}>"

    def __hash__(self):
        return hash(self.id)
    
    def __eq__(self, o):
        if isinstance(o, Player):
            return self.id == o.id
        else:
            return self.id == o

    def __lt__(self,o):
        if isinstance(o,Player):
            return self.role < o.role
        else:
            raise TypeError

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        d = self.__dict__.copy()
        d["role"] = d["role"].name
        return d

    @classmethod
    def fromdict(cls, d):
        d["role"] = Role(d["role"])
        return Player(**d)

    def to_tuple(self):
        return (self.id, "name", self.role.name, "")


class ContractingPlayer(Player, MafiaGameEncodable):
    def __init__(self, id:PlayerID, role:Role, charge:PlayerID):
        super().__init__(id,role)
        self.charge = charge
    
    def __repr__(self):
        s = super().__repr__()
        return f"{s[0:-1]},{self.charge}{s[-1:]}"

    def to_tuple(self):
        return (self.id, "name", self.role.name, self.charge)

class ChatHandle(MafiaGameEncodable):
    def __init__(self, id):
        self.id = id

    def __repr__(self):
        return str(self.id)

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        return {f"__{self.__class__.__name__}__":self.id}

    @classmethod
    def fromdict(cls, int):
        return cls(int)

class Phase(MafiaGameEncodable, Enum):
    INIT = "INIT"
    DAY = "DAY"
    NIGHT = "NIGHT"
    DUSK = "DUSK"
    END = "END"

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        return {f"__{self.__class__.__name__}__" : self.value}

    @classmethod
    def fromdict(cls, str):
        return cls(str)        

class Round(MafiaGameEncodable):

    def __init__(self, day:int=0, phase:Phase=Phase.INIT, start=None, **kwargs):
        self.day : int = day
        self._phase : Phase = None
        if not start:
            start = datetime.now()
        self.start = start
        self.phase = phase
    
    @property
    def phase(self):
        return self._phase

    @phase.setter
    def phase(self, new_phase):
        if isinstance(new_phase, tuple):
            new_phase, idiot, voters = new_phase
        if self._phase == Phase.DAY:
            del self.votes
        elif self._phase == Phase.NIGHT:
            del self.targets
            del self.mafia_target
        elif self._phase == Phase.DUSK:
            del self.idiot
            del self.votes

        if new_phase == Phase.DAY:
            self.votes : Dict[PlayerID,PlayerID] = {}
        if new_phase == Phase.NIGHT:
            self.targets : Dict[PlayerID,PlayerID] = {}
            self.mafia_target : Optional[PlayerID] = None
        if new_phase == Phase.DUSK:
            self.idiot : PlayerID = idiot
            self.voters : List[PlayerID] = voters

        self._phase = new_phase


    def __repr__(self):
        r = f"{self.__class__.__name__}:{self.phase} {self.day} @({self.start})"
        if self.phase == Phase.DAY:
            r += f" votes: {self.votes}"
        elif self.phase == Phase.NIGHT:
            r += f" targets: {self.targets}, mafia_target: {self.mafia_target}"
        elif self.phase == Phase.DUSK:
            r += f" idiot: {self.idiot}, voters: {self.voters}"
        return r

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        d = self.__dict__.copy()
        d["start"] = d["start"].strftime(DATETIME_DEFAULT_FMT)
        d["phase"] = d["_phase"].name
        del d["_phase"]
        if "voters" in d:
            d["voters"] = list(d["voters"])
        return d

    @classmethod
    def fromdict(cls, d):
        if not "phase" in d:
            raise json.JSONDecodeError("No 'phase' key in Round object")
        if not "start" in d:
            raise json.JSONDecodeError("No 'start' key in Round object")
        d["_phase"] = Phase(d["phase"])
        del d["phase"]
        d["start"] = datetime.strptime(d["start"], DATETIME_DEFAULT_FMT)
        if "votes" in d:
            d["votes"] = dict([(int(k),v) for k,v in d["votes"].items()])
        if "targets" in d:
            d["targets"] = dict([(int(k),v) for k,v in d["targets"].items()])
        if "voters" in d:
            d["voters"] = set(d["voters"])
        return super().fromdict(d)

class MafiaGameState(MafiaGameEncodable):

    def __init__(self):

        self.lobby_chat : ChatHandle = ChatHandle(None)
        self.game_number : Optional[int] = 0

        self.main_chat : ChatHandle = ChatHandle(None)
        self.mafia_chat : ChatHandle = ChatHandle(None)
        
        self.players : Set[Player] = set()

        self.round : Round = Round()

        self.rules = None

    def getPlayer(self, p_id: PlayerID):
        ps = [p for p in self.players if p.id == p_id]
        if len(ps) != 1:
            raise KeyError(f"{self.players}|{p_id}")
        return ps[0]

    def __repr__(self):
        return (f"<{self.__class__.__name__}:{self.game_number} lobby-{self.lobby_chat},"
                f" main-{self.main_chat}, mafia-{self.mafia_chat}\n"
                f"  Round:   {self.round}\n"
                f"  Players: {self.players}\n"
                f"  Rules:   {self.rules}")

    def mafia_game_encode(self):
        logging.debug(f"Encoding {self.__class__.__name__}")
        d = self.__dict__.copy()
        d["players"] = [p.mafia_game_encode() for p in self.players]
        return {f"__{self.__class__.__name__}__":d}

    @classmethod
    def fromdict(cls, d):
        if not "players" in d or not "round" in d:
            raise json.JSONDecodeError
        d["players"] = set([Player.fromdict(pd) for pd in d["players"]])
        d["round"] = Round.fromdict(d["round"])
        return super().fromdict(d)

    def save(self, fname):
        def default(o: Any) -> Any:
            logging.debug(f"Default: {o}")
            if isinstance(o, MafiaGameEncodable):
                return o.mafia_game_encode()
            return json.JSONEncoder.default(o)
        
        logging.info(f"Saving game to {fname}: {self}")
        with open(fname, "w") as fp:
            s = json.dumps(self, default=default, indent=2)
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

def eliminate(m:MafiaGameState, p_id:PlayerID):
    p = m.getPlayer(p_id)
    # TODO: Check for victory
    logging.info(f"{p_id} died")

def to_day(m : MafiaGameState):
    # Collect stuns
    stuns = set()
    for p in m.players:
        if p.role == Role.STRIPPER and \
           p.id in m.round.targets and \
           m.round.targets[p.id] not in [None, NOTARGET]:
            stuns.add(m.round.targets[p.id])
    
    # TODO: Send out stun messages if rule
    logging.info("Stun messages")

    # Collect Saves
    saves = dict([(pat,doc) for doc,pat in m.round.targets.items() \
        if m.getPlayer(doc).role == Role.DOCTOR and not doc in stuns])

    # Check kill
    if m.round.mafia_target == NOTARGET:
        # TODO: Send NOTARGET message
        logging.info("mafia NOTARGET message")
    else:
        if m.round.mafia_target in saves:
            # TODO: Send No Kill message and savior messages
            logging.info("Doc savior messages")
            logging.info("mafia NOTARGET message")
        else:
            # TODO: Send Death messages
            logging.info("Kill message")
            eliminate(m, m.round.mafia_target)
        
    # Investigate
    for cop in m.players:
        if cop.role == Role.COP and not cop in stuns:
            logging.info(f"{cop} investigates {m.round.targets[cop]}")
    
    m.round.phase = Phase.DAY
            

def check_to_day(m : MafiaGameState):
    if m.round.phase != Phase.NIGHT:
        raise TypeError(f"Cannot check_to_day if not Phase.NIGHT: {m.round.phase}")
    
    # Collect targeting role players
    targeting_players = [p.id for p in m.players if p.role.targeting]
    if all([tp in m.round.targets for tp in targeting_players]) and \
        m.round.mafia_target != None:
        
        to_day(m)



if __name__ == "__main__":

    p = Player(0, Role.TOWN)
    tp = Player(1, Role.COP)
    cp = Player(2, Role.GUARD, charge=1)

    m = MafiaGameState()

    m.lobby_chat= ChatHandle(0)
    m.game_number = 1
    m.main_chat = ChatHandle(-1)
    m.mafia_chat = ChatHandle(-2)

    m.players.update([p,tp,cp])

    m.round.phase = Phase.NIGHT
    m.round.targets[p.id] = tp.id
    logging.debug(m.save("test.maf"))
    
    m2 = MafiaGameState.load("test.maf")

    check_to_day(m2)

    # logging.debug(m)
    logging.debug(m2)