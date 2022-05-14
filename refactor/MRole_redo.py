# Import chat interfaces?

from typing import NewType

#TEMP
class MPlayer:
    def __init__(self):
        role = MRole()
#/TEMP


class MTeam:
    pass

class Town(MTeam):
    pass
class Mafia(MTeam):
    pass
class Rogue(MTeam):
    pass

# Might also be called player? Base class for anyone playing
class MRole:
    def __init__(self):
        raise NotImplementedError

    @staticmethod
    def investigate(role, level):
        return role.Team

    
class TargetingRole:
    priority = 0 # Lower goes firsts
    def action(self, game, actor:MPlayer, target:MPlayer):
        """Execute targeting action at dawn"""
        raise NotImplementedError

class TOWN:
    Team = Town

class COP(TOWN, TargetingRole):
    priority = 3
    def action(self, game, actor:MPlayer, target:MPlayer):
        """Investigate target"""
        investigation = MRole.investigate(target.role, 0)


class MAFIA:
    Team = Mafia