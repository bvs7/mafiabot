
from typing import Iterable
import random

from .MafiaState import *

__all__=["roleGen"]

DEFAULT_ROLES = [
    Role.TOWN, Role.COP, Role.MAFIA, Role.DOCTOR, Role.CELEB,
    Role.GODFATHER
]

def roleGen(players:Iterable[PlayerID], debug=False):
    roles = DEFAULT_ROLES[:len(players)]
    if not debug:
        random.shuffle(roles)
    return zip(players, roles)

