
from .MResp import MRespType
from .MState import *
from .MRules import MRules

from typing import Union
from enum import Enum, auto

class MReqType(Enum):
  VOTE = auto()
  STATUS = auto()
  HELP = auto()
  LEAVE = auto()
  TIMER = auto()
  UNTIMER = auto()
  TARGET = auto()
  OPTIONS = auto()
  REVEAL = auto()
  START = auto()
  IN = auto()
  OUT = auto()
  WATCH = auto()
  RULE = auto()
  STATS = auto()


""" MHandler is given a request, validates it, then performs any events it makes
"""
class MHandler:

  MAIN_REQUESTS = {
    MReqType.VOTE: handle_MAIN_VOTE,
    MReqType.STATUS: handle_MAIN_STATUS,
    MReqType.HELP: handle_HELP,
    MReqType.LEAVE: handle_MAIN_LEAVE,
    MReqType.TIMER: handle_MAIN_TIMER,
    MReqType.UNTIMER: handle_MAIN_UNTIMER,
  }

  def __init__(self):
    self.request_queue = []
    # Create thread to run queue?

    self.mrules = MRules()

  def handle(self, req_type : MReqType, chat_id : str, **kwargs):
    # branch on every type of request this can be, perform resp/event
    
    # Find what type of chat this was in. Lobby? DM? Main? Mafia?
    mstate = None
    chat_type = None
    reqs = None
    if chat_type == "MAIN":
      reqs = self.MAIN_REQUESTS

    if req_type not in reqs:
      mstate.mresp(MRespType.UNKNOWN_REQ, req_type=req_type.name, chat_type=chat_type)
    else:
      reqs[req_type](mstate=mstate, chat_id=chat_id, **kwargs)

  def handle_MAIN_VOTE(self, mstate, **kwargs):
    try:
      voter_id = kwargs['player_id']
      votee_id = kwargs['votee_id']
      assert voter_id in mstate.players, "{voter_id} can't vote, they aren't playing/alive"
      assert mstate.phase == DAY, "Must vote during {} phase".format(DAY)
      assert mstate.players[voter_id].vote != votee_id, "You are already voting for {target_id}"

      mstate.vote(voter_id, votee_id)

    except AssertionError as e:
      msg = str(e)
      mstate.resp(MRespType.VOTE_ERROR, reason=msg.format(**kwargs))

  def handle_MAIN_STATUS(self, mstate, **kwargs):
    mstate.resp(MRespType.MAIN_STATUS, mstate=mstate)

  def handle_HELP(self, mstate, **kwargs):
    # do routing for lobby/dms, etc
    mresp = mstate.mresp

    # Get next word, route here

  def handle_MAIN_LEAVE(self, mstate, **kwargs):
    pass

  def handle_MAIN_TIMER(self, mstate, **kwargs):
    pass

  def handle_MAIN_UNTIMER(self, mstate, **kwargs):
    pass