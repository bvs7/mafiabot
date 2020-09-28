from enum import Enum, auto
from typing import List, Union, Optional, Dict, Callable, Any
from threading import Lock, Thread
import json

from .MPlayer import MPlayerID

class MVengeance:
  venges : List[MPlayerID]
  final_vote : MPlayerID
  idiot : MPlayerID

  def __init__(self, venges : List[MPlayerID], final_vote : MPlayerID, idiot : MPlayerID):
    self.venges = venges
    self.final_vote = final_vote
    self.idiot = idiot

  def to_json(self):
    d = {
      'venges':self.venges,
      'final_vote':self.final_vote,
      'idiot':self.idiot,
    }
    return d

  @staticmethod
  def from_json(d):
    return MVengeance(d['venges'], d['final_vote'], d['idiot'])
