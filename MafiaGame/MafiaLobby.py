
from typing import Set


from .MafiaSaves import *
from .MafiaRules import *
from .MafiaState import *

class MafiaLobby(MafiaEncodable):

    def __init__(self, chat:ChatHandle=None, rules:RuleSet=None, inset:Set[PlayerID]=None):
        self.chat : ChatHandle = chat
        self.rules = RuleSet() if rules is None else rules
        self.inset = set() if inset is None else inset
        self.prep_msg_id = None

    def to_dict(self):
        d = super().to_dict()
        d["inset"] = list(d["inset"])
        return d

    @classmethod
    def from_dict(self, d):
        d["chat"] = ChatHandle.from_dict(d["chat"])
        d["rules"] = RuleSet.from_dict(d["rules"])
        d["inset"] = set(PlayerID(pd) for pd in d["inset"])
        return super().from_dict(d)