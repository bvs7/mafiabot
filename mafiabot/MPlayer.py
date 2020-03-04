# from MTarget import NullTarget, NoTarget, PlayerTarget

# from enum import Enum
from typing import Optional
from .MEx import MPlayerID

TOWN_ROLES = [
  'TOWN',
  'COP',
  'DOCTOR',
  'CELEB',
  'MILLER',
  'MILKY',
  'MASON',
]

MAFIA_ROLES = [
  'MAFIA',
  'GODFATHER',
  'STRIPPER',
  'GOON',
]

ROGUE_ROLES = [
  'IDIOT',
  'SURVIVOR',
  'GUARD',
  'AGENT',
]

ALL_ROLES = TOWN_ROLES + MAFIA_ROLES + ROGUE_ROLES

TARGETING_ROLES = {
  'COP',
  'DOCTOR',
  'MILKY',
  'STRIPPER',
}

CONTRACT_ROLES = {
  'IDIOT',
  'SURVIVOR',
  'GUARD',
  'AGENT',
}

class MPlayer:
  def __init__(
    self, id : MPlayerID, role : str, vote : Optional[MPlayerID]=None, target: Optional[MPlayerID]=None):

    self.id = id
    self.vote : Optional[MPlayerID] = vote
    self.role : str = role
    self.target : Optional[MPlayerID] = target

  def __str__(self):
    return "[{id},{role}:{vote}:{target}]".format(**self.__dict__)
    
  def __repr__(self):
    return "[{id},{role}:{vote}:{target}]".format(**self.__dict__)

