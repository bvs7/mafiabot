
from typing import Any

import logging
import json

logging.basicConfig(level=logging.INFO)

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

class MafiaGameDecoder(json.JSONDecoder):

    @staticmethod
    def save(obj: MafiaGameEncodable, fname):
        def default(o: Any) -> Any:
            logging.debug(f"Default: {o}")
            if isinstance(o, MafiaGameEncodable):
                return o.mafia_game_encode()
            return json.JSONEncoder.default(o)
        
        logging.info(f"Saving game to {fname}: {self}")
        with open(fname, "w") as fp:
            s = json.dumps(obj, default=default, indent=2)
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