
from .MafiaSaves import *
from .MafiaRules import *
from .MafiaState import *

class MafiaLobby(MafiaEncodable):

    def __init__(self, rules:RuleSet=None):
        self.rules = RuleSet() if rules is None else rules

    @classmethod
    def from_dict(self, d):
        d["rules"] = RuleSet.from_dict(d["rules"])
        return super().from_dict(d)