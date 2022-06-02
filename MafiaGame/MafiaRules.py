
from enum import Enum, EnumMeta

from .MafiaSaves import MafiaEncodable, MafiaEnumEncodable

__all__ = [
    "Rule", "StartNight", "SavePublic", "SavePrivate", "DeathReveal",
    "StartReveal", "Investigate", "IdiotEvent", "RuleSet"
]

class Rule(MafiaEnumEncodable, Enum):

    def __new__(cls, value, default=False):
        obj = object.__new__(cls)
        obj._value_ = value
        if default:
            cls.default = obj
        return obj

    def __eq__(self, other):
        if not isinstance(other, Rule):
            return False
        if not self.__class__ == other.__class__:
            return False
        return self.name == other.name


class StartNight(Rule):
    EVEN = "EVEN", True
    ODD = "ODD"
    NEVER = "NEVER"
    ALWAYS = "ALWAYS"

class SavePublic(Rule):
    NONE = "NONE"
    ANON = "ANON", True
    PATIENT = "PATIENT"
    ALL = "ALL"

class SavePrivate(Rule):
    NONE = "NONE", True
    DOCTOR = "DOCTOR",
    PATIENT = "PATIENT",
    BOTH = "BOTH"

class DeathReveal(Rule):
    NONE = "NONE"
    MAFIA = "MAFIA", True
    TEAM = "TEAM"
    ROLE = "ROLE"

class StartReveal(Rule):
    NONE = "NONE"
    MAFIA = "MAFIA"
    TEAM = "TEAM", True
    ROLE = "ROLE"

class Investigate(Rule):
    MAFIA = "MAFIA", True
    TEAM = "TEAM"
    ROLE = "ROLE"

class IdiotEvent(Rule):
    NONE = "NONE"
    STUN = "STUN"
    DUSK = "DUSK", True
    CULL = "CULL"
    WIN = "WIN"

class RuleSet(MafiaEncodable):

    def __init__(self,
                 startNight=StartNight.default,
                 savePublic=SavePublic.default,
                 savePrivate=SavePrivate.default,
                 deathReveal=DeathReveal.default,
                 startReveal=StartReveal.default,
                 investigate=Investigate.default,
                 idiotEvent=IdiotEvent.default):
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
            return self.__dict__ == other.__dict__

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