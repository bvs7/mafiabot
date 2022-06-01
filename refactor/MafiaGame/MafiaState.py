
from typing import List, Dict, Any, Optional, NewType, Set, Iterable
from collections import namedtuple
from enum import Enum
from datetime import datetime
import logging

logging.basicConfig(level=logging.DEBUG)

DATETIME_DEFAULT_FMT = "%Y-%m-%d %H:%M:%S.%f"


from .MafiaSaves import MafiaEncodable, MafiaEnumEncodable
from .MafiaRules import RuleSet

__all__=[
    "Team", "Role", "Phase", "PlayerID", "ChatHandle",
    "Player", "Round", "GameState", "GameEndException"
]

class Team(MafiaEnumEncodable):
    Town = "Town"
    Mafia = "Mafia"
    Rogue = "Rogue"

    def __str__(self):
        return self.name

    def __eq__(self, other):
        if not isinstance(other, Team):
            return False
        return self.name == other.name


class Role(MafiaEnumEncodable):
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

    def __eq__(self, other):
        if not isinstance(other, Role):
            return False
        return self.name == other.name

    def __lt__(self, other):
        if not isinstance(other, Role):
            raise ValueError
        k = list(Role.__dict__.keys())
        return k.index(self.name) < k.index(other.name)

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

class Player(MafiaEncodable):
    
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

class ChatHandle(MafiaEncodable):
    def __init__(self, id):
        self.id = id

    def __repr__(self):
        return str(self.id)

    def __eq__(self, other):
        if not isinstance(other, ChatHandle):
            return False
        return self.id == other.id

    def to_dict(self):
        return self.id

    @classmethod
    def from_dict(cls, int):
        return cls(int)

class Phase(MafiaEnumEncodable):
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
     

class Round(MafiaEncodable):

    def __init__(self, day:int=0, phase:Phase=Phase.INIT, start=None, **kwargs):
        self.day : int = day
        self._phase = Phase.INIT
        self.set_phase(phase, **kwargs)
        if not start:
            start = datetime.now()
        self.start = start

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
                self.votes : Dict[PlayerID,PlayerID] = {} if not "votes" in kw \
                    else dict([(PlayerID(p),PlayerID(v)) for (p,v) in kw["votes"].items()])
            elif new_phase == Phase.NIGHT:
                self.targets : Dict[PlayerID,PlayerID] = {} if not "targets" in kw \
                    else dict([(PlayerID(p),PlayerID(t)) for (p,t) in kw["targets"].items()])
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

    def __eq__(self, other):
        if not isinstance(other, Round):
            return False
        if self.phase != other.phase:
            return False
        return self.__dict__ == other.__dict__
    
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
        d["start"] = datetime.strptime(d["start"], DATETIME_DEFAULT_FMT)
        if Phase.DUSK == d["phase"]:
            d["voters"] = set(d["voters"])
        return super().from_dict(d)

class GameState(MafiaEncodable):

    def __init__(self, 
        lobby_chat=None, game_number=None, main_chat=None, mafia_chat=None,
        players=None, round=None, rules=None):

        self.lobby_chat : ChatHandle = lobby_chat
        self.game_number : Optional[int] = game_number

        self.main_chat : ChatHandle = main_chat
        self.mafia_chat : ChatHandle = mafia_chat
        
        self.players : Set[Player] = set() if players == None else players

        self.round : Round = Round() if round == None else round

        self.rules = RuleSet() if rules == None else rules

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

    def __eq__(self, other):
        if not isinstance(other, GameState):
            return False
        return self.__dict__ == other.__dict__

    @classmethod
    def from_dict(cls, d):
        d["players"] = set([Player.from_dict(pd) for pd in d["players"]])
        d["round"] = Round.from_dict(d["round"])
        d["rules"] = RuleSet.from_dict(d["rules"])
        d["lobby_chat"] = ChatHandle.from_dict(d["lobby_chat"])
        d["main_chat"] = ChatHandle.from_dict(d["main_chat"])
        d["mafia_chat"] = ChatHandle.from_dict(d["mafia_chat"])
        return super().from_dict(d)

class GameEndException(Exception):
    
    def __init__(self, m:GameState, *args, winner=None, **kwargs):
        self.winner = winner
        super().__init__(*args, **kwargs)

        m.round.phase = (Phase.END, winner)
