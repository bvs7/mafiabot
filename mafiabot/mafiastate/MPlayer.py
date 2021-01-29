# from MTarget import NullTarget, NoTarget, PlayerTarget
from .MRole import MRole

# from enum import Enum
from typing import Optional, NewType

# TODO: how to generalize this to a subclass? or subsystem?
MPlayerID = NewType('MPlayerID', str)

NOTARGET : MPlayerID = "NOTARGET"

class MPlayer:
  MPlayerID = MPlayerID
  NOTARGET : MPlayerID = "NOTARGET"

  def __init__(self, 
      id : MPlayerID, 
      role : MRole, 
      vote : Optional[MPlayerID]=None, 
      target: Optional[MPlayerID]=None
    ):

    self.id = id
    self.vote : Optional[MPlayerID] = vote
    self.role : MRole = role
    self.target : Optional[MPlayerID] = target

  def __str__(self):
    return "[{id},{role}:{vote}:{target}]".format(**self.__dict__)
    
  def __repr__(self):
    return "[%s,%s:%s:%s]" % (self.id, repr(self.role), self.vote, self.target)

  def to_json(self):
    d = {
      "id":self.id,
      "role":self.role,
    }
    if not self.vote == None:
      d["vote"] = self.vote
    if not self.target == None:
      d["target"] = self.target

    return d

  @staticmethod
  def from_json(d):
    if not 'vote' in d:
      d['vote'] = None
    if not 'target' in d:
      d['target'] = None
    return MPlayer(d['id'],d['role'],d['vote'],d['target'])

