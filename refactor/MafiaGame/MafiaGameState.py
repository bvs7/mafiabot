
from typing import List, Dict, Any, Optional, NewType, Set, Iterable
from collections import namedtuple
from enum import Enum, EnumMeta

import json

from datetime import datetime

DATETIME_DEFAULT_FMT = "%Y-%m-%d %H:%M:%S.%f"

import logging

logging.basicConfig(level=logging.DEBUG)

class MafiaGameEncoder(json.JSONEncoder):

    def __init__(self,  *args, **kwargs):
        kwargs["default"] = self.default
        self.root = True
        json.JSONEncoder.__init__(self, *args, **kwargs)

    def default(self, o):
        if isinstance(o, MafiaGameEncodableBase):
            r =  o.mafia_game_encode(self.root)
            self.root = False
            return r
        return json.JSONEncoder.default(self, o)

    def encode(self, o):
        logging.debug(f"Encode {self.__class__}")
        if isinstance(o, MafiaGameEncodableBase):
            o = o.mafia_game_encode(True)
        return super().encode(o)


class MafiaGameDecoder(json.JSONDecoder):

    def __init__(self, *args, **kwargs):
        self.decodables = MafiaGameEncodable.__subclasses__() + MafiaGameEncodableBase.__subclasses__()
        kwargs["object_hook"] = self.object_hook
        json.JSONDecoder.__init__(self, *args, **kwargs)

    def object_hook(self, d):
        """Checks for explicit MafiaGameEncodable Objects"""
        logging.debug(f"obj hook {d}")
        for cls in self.decodables:
            class_str = f"__{cls.__name__}__"
            if class_str in d:
                logging.debug(f"Classfromdict {cls}")
                return cls.from_dict(d[class_str])
        return d


class MafiaGameEncodableBase:

    def mafia_game_encode(self, root=False):
        logging.debug(f"{self.__class__}, {root}")
        d = self.to_dict()
        if root:
            logging.debug(f"Encoding root class: {self.__class__}")
            d = {f"__{self.__class__.__name__}__":d}
        return d

    def to_dict(self):
        d = self.__dict__.copy()
        def convert(o):
            if isinstance(o, MafiaGameEncodableBase):
                return o.mafia_game_encode(False)
            return o
        for k,v in d.items():
            if isinstance(v, tuple) or isinstance(v, list) or isinstance(v, set):
                d[k] = [convert(vi) for vi in v]
            elif isinstance(v, dict):
                for vk, vv in v.items():
                    v[vk] = convert(vv)
            else:
                d[k] = convert(v)

        return d


    @classmethod
    def from_dict(cls, d):
        logging.debug(f"Fromdict: {cls}, {d}")
        return cls(**d)

    def save(self, fname=None):
        if fname == None:
            return json.dumps(self, cls=MafiaGameEncoder, indent=2)
        with open(fname, "w") as fp:
            return json.dump(self, fp, cls=MafiaGameEncoder, indent=2)

    @classmethod
    def load(c, fname=None, s=None):
        if not fname:
            if not s:
                raise TypeError("No fname or string specified")
            o = json.loads(s, cls=MafiaGameDecoder)
        with open(fname) as fp:
            o = json.load(fp, cls=MafiaGameDecoder)
        
        if not isinstance(o, c):
            raise TypeError(f"Loaded incorrect object, should be {c} but got {o.__class__}: {o} ")
        return o

class MafiaGameEncodable(MafiaGameEncodableBase):

    def __new__(cls, *args, **kwargs):
        logging.debug(f"New ... {cls, args, kwargs}")
        if len(args) >= 1:
            if isinstance(args[0], dict):
                logging.debug(f"New fromdict... {cls}")
                o = cls.from_dict(args[0])
                o.init = True
                return o
            elif isinstance(args[0], cls):
                logging.debug(f"Copy constr")
                o = args[0] # copy constructor doesn't duplicate yet!
                o.init = True
                return o
        return super(MafiaGameEncodable, cls).__new__(cls)


class VEnumMeta(EnumMeta):

    def __call__(cls, value, *args, **kwargs):
        if value is None:
            return cls.default
        return super().__call__(value, *args, **kwargs)

    def _create_(cls, class_name, names=None, *, module=None, qualname=None, type=None, start=1):
        n0 = names[0]
        names2 = [(n,n) for n in names]
        enum_class = super()._create_(class_name, names=names2, 
            module=module, qualname=qualname, type=type, start=start)
        enum_class.default = enum_class(n0)
        return enum_class

class VEnum(Enum, metaclass=VEnumMeta):
    
    def __eq__(self, other):
        if isinstance(other, str):
            return self.name == other
        return super().__eq__(other)

StartNight =  VEnum("StartNight", ["EVEN", "ALWAYS","NEVER","ODD"])
SavePublic =  VEnum("SavePublic", ["NONE", "ANON", "PATIENT", "ALL"])
SavePrivate = VEnum("SavePrivate", ["NONE", "DOC", "PATIENT", "DOC_PATIENT"])
DeathReveal = VEnum("DeathReveal", ["MAF", "NONE", "TEAM", "ROLE"])
StartReveal = VEnum("StartReveal", ["TEAM", "NONE", "MAF", "ROLE"])
Investigate = VEnum("Investigate", ["MAF", "TEAM", "ROLE"])
IdiotEvent =  VEnum("IdiotEvent", ["DUSK", "NONE", "STUN", "CULL" ,"WIN"])

class RuleSet(MafiaGameEncodable):

    def __init__(self, *args,
        startNight=None, savePublic=None, savePrivate=None, deathReveal=None,
        startReveal=None, investigate=None, idiotEvent=None):
        self.startNight = StartNight(startNight)
        self.savePublic = SavePublic(savePublic)
        self.savePrivate = SavePrivate(savePrivate)
        self.deathReveal = DeathReveal(deathReveal)
        self.startReveal = StartReveal(startReveal)
        self.investigate = Investigate(investigate)
        self.idiotEvent = IdiotEvent(idiotEvent)

    def __repr__(self):
        return ", ".join(
            [f"{r.__class__.__name__}:{r.name}" for r in self.__dict__.values()]
        )

    def __eq__(self, other):
        if isinstance(other, RuleSet):
            return all([self.__dict__[k] == other.__dict__[k] for k in self.__dict__])

    def __iter__(self):
        return iter(self.__dict__.items())

    def __getitem__(self, key):
        return self.__dict__[key]

    def __setitem__(self, k, v):
        ktype = self.__dict__[k].__class__
        self.__dict__[k] = ktype(v)

    def to_dict(self):
        if self == RuleSet():
            d = "default"
        else:
            d = self.__dict__.copy()
            for k in d:
                d[k] = d[k].name
        return d

    @classmethod
    def from_dict(cls, d):
        if isinstance(d, dict):
            return cls(**d)
        elif isinstance(d, str) and d == "default": 
            return cls()
        raise NotImplementedError

class Team(MafiaGameEncodableBase, Enum):
    Town = "Town"
    Mafia = "Mafia"
    Rogue = "Rogue"

    def to_dict(self):
        return self.name

    @classmethod
    def from_dict(cls, str):
        return cls(str)

    def __str__(self):
        return self.name

# Might also be called player? Base class for anyone playing
class Role(MafiaGameEncodableBase, Enum):
    TOWN = "TOWN",      Team.Town
    COP = "COP",        Team.Town, "targeting"
    DOCTOR = "DOCTOR",  Team.Town, "targeting"
    CELEB = "CELEB",    Team.Town
    MILLER = "MILLER",  Team.Town
    MILKY = "MILKY",    Team.Town, "targeting"

    MAFIA = "MAFIA",        Team.Mafia
    GODFATHER = "GODFATHER",Team.Mafia
    STRIPPER = "STRIPPER",  Team.Mafia, "targeting"
    GOON = "GOON",          Team.Mafia

    IDIOT = "IDIOT",    Team.Rogue
    SURVIVOR="SURVIVOR",Team.Rogue
    GUARD = "GUARD",    Team.Rogue, "contracting"
    AGENT = "AGENT",    Team.Rogue, "contracting"

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

    def to_dict(self):
        return self.name

    @classmethod
    def from_dict(cls, str):
        return cls(str)  

class PlayerID(int):
    
    def __new__(cls, *args, **kwargs):
        if len(args) >= 1 and args[0] == None:
            return PlayerID.NONE
        return super().__new__(cls, *args, **kwargs)

    @classmethod
    @property
    def NOTARGET(cls):
        return cls(-1)
    
    @classmethod
    @property
    def NONE(cls):
        return cls(0)


class Player(MafiaGameEncodable):
    
    def __init__(self, id:PlayerID, role:Role = Role.TOWN, charge = None):
        self.id = PlayerID(id)
        assert self.id not in [PlayerID.NOTARGET, PlayerID.NONE]
        if isinstance(role, list) or isinstance(role, tuple):
            role, charge = role
        self.role = role, PlayerID(charge)
    
    def __repr__(self):
        result = f"<{self.__class__.__name__}:{self.id},{self.role.name}>"
        if self.role.contracting:
            result = result[:-1] + f"({self.charge})" + result[-1:]
        return result

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

    @property
    def role(self):
        return self._role

    @role.setter
    def role(self, r):
        charge = None
        if isinstance(r, tuple) or isinstance(r,list):
            charge = r[1]
            r = r[0]
        if isinstance(r, str):
            try:
                r = Role(r)
            except ValueError as ve:
                logging.error(f"Error setting role to {r}: {ve}")
                return
        if isinstance(r, Role):
            self.charge = charge
            self._role = r
            return
        raise NotImplementedError(r)

    def to_dict(self):
        d = self.__dict__.copy()
        d["role"] = d["_role"].name
        del d["_role"]
        if not self.role.contracting:
            del d["charge"]
        return d

    @classmethod
    def from_dict(cls, d):
        return Player(**d)

    def to_tuple(self):
        return (self.id, "name", self.role.name, self.charge)

class ChatHandle(MafiaGameEncodable):
    def __init__(self, id):
        self.id = id

    def __repr__(self):
        return str(self.id)

    def to_dict(self):
        return self.id

    @classmethod
    def from_dict(cls, int):
        return cls(int)

class Phase(MafiaGameEncodableBase, Enum):
    INIT = "INIT"
    DAY = "DAY"
    NIGHT = "NIGHT"
    DUSK = "DUSK"
    END = "END"

    def __eq__(self, other):
        if isinstance(other, str):
            return self.name == other
        else:
            return super().__eq__(other)

    def to_dict(self):
        return self.value

    @classmethod
    def from_dict(cls, str):
        return cls(str)        

class Round(MafiaGameEncodable):

    def __init__(self, day:int=0, phase:Phase=Phase.INIT, start=None, **kwargs):
        if hasattr(self, "init") and self.init:
            del self.init
            return
        logging.debug(f"Round init: {day, phase, start, kwargs}")
        self.day : int = day
        self._phase = Phase.INIT # DO NOT SET DIRECTLY
        if not start:
            start = datetime.now()
        self.start = start
        self.set_phase(phase, **kwargs)

    def set_phase(self, new_phase, **kw):
        new_phase = Phase(new_phase)
        if self._phase != new_phase:
            if self._phase == Phase.DAY:
                del self.votes
            elif self._phase == Phase.NIGHT:
                del self.targets
                del self.mafia_target
            elif self._phase == Phase.DUSK:
                del self.idiot
                del self.voters
            elif self._phase == Phase.END:
                del self.winner

            if new_phase == Phase.DAY:
                self.votes : Dict[PlayerID,PlayerID] = {}
                self.day += 1
            elif new_phase == Phase.NIGHT:
                self.targets : Dict[PlayerID,PlayerID] = {} if not "targets" in kw \
                    else [(PlayerID(p),PlayerID(t)) for (p,t) in kw["targets"].items()]
                self.mafia_target : Optional[PlayerID] = None if \
                    not "mafia_target" in kw else kw["mafia_target"]
            elif new_phase == Phase.DUSK:
                self.idiot : PlayerID = PlayerID(kw["idiot"]) # required args
                self.voters : Set[PlayerID] = set(kw["voters"])
            elif new_phase == Phase.END:
                self.winner = Team(kw["winner"])


        self._phase = new_phase

    @property
    def phase(self):
        return self._phase

    @phase.setter
    def phase(self, new_phase, **kwargs):
        self.set_phase(new_phase, **kwargs)

    def __repr__(self):
        r = f"{self.__class__.__name__}:{self.phase} {self.day} @({self.start})"
        if self.phase == Phase.DAY:
            r += f" votes: {self.votes}"
        elif self.phase == Phase.NIGHT:
            r += f" targets: {self.targets}, mafia_target: {self.mafia_target}"
        elif self.phase == Phase.DUSK:
            r += f" idiot: {self.idiot}, voters: {self.voters}"
        return r
    
    def to_dict(self):
        d = super().to_dict()
        d["start"] = d["start"].strftime(DATETIME_DEFAULT_FMT)
        d["phase"] = d["_phase"]
        del d["_phase"]
        if self.phase == Phase.DUSK:
            d['voters'] = list(d['voters'])
        return d

    @classmethod
    def from_dict(cls, d):
        logging.debug(f"Round fromdict: {d}")
        d["start"] = datetime.strptime(d["start"], DATETIME_DEFAULT_FMT)
        if Phase.DUSK == d["phase"]:
            d["voters"] = set(d["voters"])
        return super().from_dict(d)


class MafiaGameState(MafiaGameEncodable):

    def __init__(self, 
        lobby_chat=None, game_number=None, main_chat=None, mafia_chat=None,
        players=None, round=None, rules=None):

        self.lobby_chat : ChatHandle = ChatHandle(lobby_chat)
        self.game_number : Optional[int] = game_number

        self.main_chat : ChatHandle = ChatHandle(main_chat)
        self.mafia_chat : ChatHandle = ChatHandle(mafia_chat)
        
        # Players will be list? Could be dict of id to other player info
        self.players : Set[Player] = set() if players == None else players

        self.round : Round = Round() if round == None else Round(round)

        self.rules = RuleSet() if rules == None else RuleSet(rules)

    def getPlayer(self, p_id: PlayerID):
        ps = [p for p in self.players if p.id == p_id]
        if len(ps) != 1:
            raise KeyError(f"{self.players}|{p_id, type(p_id), ps}")
        return ps[0]

    def __repr__(self):
        return (f"<{self.__class__.__name__}:{self.game_number} lobby-{self.lobby_chat},"
                f" main-{self.main_chat}, mafia-{self.mafia_chat}\n"
                f"  Round:   {self.round}\n"
                f"  Players: {self.players}\n"
                f"  Rules:   {self.rules}")

    @classmethod
    def from_dict(cls, d):
        if not "players" in d or not "round" in d:
            raise json.JSONDecodeError
        d["players"] = set([Player.from_dict(pd) for pd in d["players"]])
        return super().from_dict(d)


class GameEndException(Exception):
    
    def __init__(self, m:MafiaGameState, *args, winner=None, **kwargs):
        self.winner = winner
        super().__init__(*args, **kwargs)

        m.round.phase = (Phase.END, winner)

if __name__ == "__main__":

    p = Player(1, Role.TOWN)
    tp = Player(2, Role.COP)
    cp = Player(3, Role.IDIOT)

    m = MafiaGameState()

    m.lobby_chat= ChatHandle(0)
    m.game_number = 1
    m.main_chat = ChatHandle(-1)
    m.mafia_chat = ChatHandle(-2)

    m.players.update([p,tp,cp])

    m.round.set_phase(Phase.NIGHT)
    m.round.targets[p.id] = tp.id
    logging.debug(m.save("test_.maf"))
    
    m2 = MafiaGameState.load("test_.maf")

    # logging.debug(m)
    logging.debug(m2)

    m2.round.set_phase(Phase.DAY, votes = {"1":"0", "2":"1"})
    logging.debug(m2.save("test_.maf"))
    
    m2 = MafiaGameState.load("test_.maf")
    logging.debug(m2)

    m2.round.set_phase(Phase.DUSK, voters = {"1","2"}, idiot="3")
    logging.debug(m2.save("test_.maf"))
    
    m2 = MafiaGameState.load("test_.maf")
    logging.debug(m2)

    m2.round.set_phase(Phase.END, winner=Team.Town)
    logging.debug(m2.save("test_.maf"))
    
    m2 = MafiaGameState.load("test_.maf")
    logging.debug(m2)