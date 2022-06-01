
import json
import enum

import logging

logging.basicConfig(level=logging.INFO)

__all__ = ["MafiaEncodable", "MafiaEnumEncodable"]

class MafiaEncoder(json.JSONEncoder):

    def __init__(self,  *args, **kwargs):
        kwargs["default"] = self.default
        json.JSONEncoder.__init__(self, *args, **kwargs)

    def default(self, o):
        if isinstance(o, MafiaEncodable):
            r =  o.mafia_game_encode()
            return r
        return json.JSONEncoder.default(self, o)

    def encode(self, o):
        if isinstance(o, MafiaEncodable):
            o = o.mafia_game_encode(True)
        return super().encode(o)

def all_subclasses(cls):
    return set(cls.__subclasses__()).union(
        [s for c in cls.__subclasses__() for s in all_subclasses(c)])

class MafiaDecoder(json.JSONDecoder):

    def __init__(self, *args, **kwargs):
        self.decodables = all_subclasses(MafiaEncodable)
        kwargs["object_hook"] = self.object_hook
        json.JSONDecoder.__init__(self, *args, **kwargs)

    def object_hook(self, d):
        """Checks for explicit MafiaEncodable Objects"""
        for cls in self.decodables:
            class_str = f"__{cls.__name__}__"
            if class_str in d:
                return cls.from_dict(d[class_str])
        return d


class MafiaEncodable:

    def mafia_game_encode(self, root=False):
        d = self.to_dict()
        if root:
            d = {f"__{self.__class__.__name__}__":d}
        return d

    def to_dict(self):
        d = self.__dict__.copy()
        for k,v in d.items():
            if isinstance(v, set):
                d[k] = list(v)
        return d

    @classmethod
    def from_dict(cls, d):
        return cls(**d)

    def save(self, fname=None):
        if fname == None:
            return MafiaEncoder(indent=2).encode(self)
        with open(fname, "w") as fp:
            fp.write( MafiaEncoder(indent=2).encode(self) )

    @classmethod
    def load(cls, fname:str=None, s:str=None):
        if fname == None:
            if not s:
                raise TypeError("No fname or string specified")
            o = MafiaDecoder().decode(s)
        else:
            with open(fname) as fp:
                o = MafiaDecoder().decode(fp.read())
        
        if not isinstance(o, cls):
            raise TypeError(f"Loaded incorrect object, should be {cls} but got {o.__class__}: {o} ")
        return o

class MafiaEnumEncodable(MafiaEncodable, enum.Enum):

    def to_dict(self):
        return self.value

    @classmethod
    def from_dict(cls, str):
        return cls(str) 