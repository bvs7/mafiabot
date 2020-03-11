
from .MResp import MRespType
from .MState import *
from .MRules import MRules
from .MTimer import MTimer

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
      voter_id = kwargs['voter_id']
      votee_id = kwargs['votee_id']
      assert voter_id in mstate.players, "{voter_id} can't vote, they aren't playing"
      assert mstate.phase == DAY, "Must vote during {} phase".format(DAY)
      assert mstate.players[voter_id].vote != votee_id, "You are already voting for {target_id}"

      mstate.vote(voter_id, votee_id)

    except AssertionError as e:
      msg = str(e)
      mstate.resp(MRespType.VOTE_ERROR, reason=msg.format(**kwargs))

  def handle_MAIN_STATUS(self, mstate, **kwargs):
    mstate.resp(MRespType.MAIN_STATUS, mstate=mstate)

  def handle_HELP(self, mstate, **kwargs):
    pass
    
    

  def handle_MAIN_LEAVE(self, mstate, **kwargs):
    pass

  def handle_MAIN_TIMER(self, mstate, **kwargs):
    try:
      player_id = kwargs['player_id']
      assert player_id in mstate.players, "{player_id} can't timer, they aren't playing"
      assert not player_id in mstate.timerers, "{player_id} has already timered"
      
      # check timer status for mstate
      n_timers = len(mstate.timerers)
      n_players = len(mstate.players)

      def timer_end():
        mstate.timer()
      
      def reminder(n):
        mstate.mresp(MRespType.TIMER_REMINDER, minutes=n)

      if n_timers == 0:
        # start timer
        mstate.timer = MTimer(5*60*n_players, [timer_end])
        mstate.timerers.append(player_id)
        mstate.mresp(MRespType.START_TIMER, player_id=player_id, time=mstate.timer.getTime() )
      else:
        mstate.timer.addTime(-5*60)
        mstate.timerers.append(player_id)
        mstate.mresp(MRespType.ADD_TIMER, player_id=player_id, time=mstate.timer.getTime())

  def handle_MAIN_UNTIMER(self, mstate, **kwargs):
    pass