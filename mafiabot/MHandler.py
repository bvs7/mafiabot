
from .MResp import MRespType
from .MState import *
from .MRules import MRules
from .MTimer import MTimer
from .MReq import MReqType

from flask import Flask, request

from typing import Union
from enum import Enum, auto

TIMER_LEN = 5*60

class MHandlerReq(Enum):
  VOTE = auto()
  MTARGET = auto()
  MOPTIONS = auto()
  TARGET = auto()
  OPTIONS = auto()
  REVEAL = auto()

  START = auto()
  IN = auto()
  OUT = auto()

  LOBBY_STATUS = auto()
  MAIN_STATUS = auto()
  MAFIA_STATUS = auto()
  DM_STATUS = auto()
  MAIN_LEAVE = auto()
  MAFIA_LEAVE = auto()
  TIMER = auto()
  UNTIMER = auto()

""" MHandler is given a request, validates it, then performs any events it makes
"""
class MHandler:

  def __init__(self):

    self.mstates = {} 

    self.REQUESTS = {}
    for i in dir(self):
      o = self.__getattribute__(i)
      if callable(o):
        if i[0:7] == 'handle_':
          self.REQUESTS[i[7:]] = o

  def handle(self, req_type : MHandlerReq, resp:MResp, **kwargs):
    req = req_type.name
    if req in self.REQUESTS:
      self.REQUESTS[req](resp, **kwargs)


  def handle_VOTE(self, resp:MResp, **kwargs):
    try:
      voter_id = kwargs['voter_id']
      votee_id = kwargs['votee_id']
      assert voter_id in self.mstate.players, "{voter_id} can't vote, they aren't playing"
      assert self.mstate.phase == DAY, "Must vote during {} phase".format(DAY)
      assert self.mstate.players[voter_id].vote != votee_id, "You are already voting for {target_id}"

      self.mstate.vote(voter_id, votee_id)

    except AssertionError as e:
      msg = str(e)
      self.mstate.mresp(MRespType.VOTE_ERROR, reason=msg.format(**kwargs))

  def handle_MTARGET(self,**kwargs):
    try:
      player_id = kwargs['player_id']
      target_id = kwargs['target_id']
      assert player_id in self.mstate.players, "[{player_id}] can't target, they aren't playing"
      assert self.mstate.phase == "Night", "Targeting can only be done at Night"
      assert self.mstate.players[player_id].role != "GOON", "[{player_id}], as a GOON, cannot target"
      assert target_id in self.mstate.players, "[{player_id}] can't target [{target_id}], [{target_id}] isn't playing"
      self.mstate.mtarget(player_id, target_id)

    except AssertionError as e:
      msg = str(e)
      self.mstate.mresp(MRespType.MTARGET_ERROR, reason=msg.format(**kwargs))

    
  def handle_MOPTIONS(self,**kwargs):
    try:
      player_id = kwargs['player_id']
      assert player_id in self.mstate.players, "[{player_id}] can't get options, they aren't playing"
      assert self.mstate.phase == "Night", "Options can only be shown at Night"

      self.mstate.mresp(MRespType.NIGHT_OPTIONS, dest="MAFIA", **kwargs)
    except AssertionError as e:
      msg = str(e)
      self.mstate.mresp(MRespType.MOPTIONS_ERROR, reason=msg.format(**kwargs))

  def handle_TARGET(self,**kwargs):
    try:
      player_id = kwargs['player_id']
      target_id = kwargs['target_id']
      role = self.mstate.players[player_id].role
      kwargs['role'] = role
      assert player_id in self.mstate.players, "You can't target, you aren't playing"
      assert self.mstate.phase == "Night", "Targeting can only be done at Night"
      assert role in TARGETING_ROLES, "[{player_id}], as a {role}, you cannot target"
      assert target_id in self.mstate.players, "You can't target [{target_id}], they aren't playing"

      self.mstate.target(player_id, target_id)

    except AssertionError as e:
      msg = str(e)
      self.mstate.mresp(MRespType.TARGET_ERROR, reason=msg.format(**kwargs), dest=player_id)
      return
    

  def handle_OPTIONS(self, **kwargs):
    try:
      player_id = kwargs['player_id']
      role = self.mstate.players[player_id].role
      assert player_id in self.mstate.players, "You can't get options, you aren't playing"
      assert self.mstate.phase == "Night", "You can only get options at Night"
      assert role in TARGETING_ROLES, "You can't get options, you aren't a targeting role"
      self.mstate.mresp(MRespType.NIGHT_OPTIONS, dest=player_id, **kwargs)

    except AssertionError as e:
      msg = str(e)
      self.mstate.mresp(MRespType.OPTIONS_ERROR, reason=msg.format(**kwargs), dest=player_id)

  def handle_REVEAL(self, **kwargs):
    try:
      player_id = kwargs['player_id']
      assert player_id in self.mstate.players, "You can't reveal, you aren't playing"
      role = self.mstate.players[player_id].role
      assert self.mstate.phase == "Day", "You can only reveal during the day"
      assert role == "CELEB", "You can only reveal if you are CELEB"
      self.mstate.reveal(player_id)
    except AssertionError as e:
      msg = str(e)
      self.mstate.mresp(MRespType.REVEAL_ERROR, dest=player_id, **kwargs)

  def handle_MAIN_STATUS(self,**kwargs):
    self.mstate.mresp(MRespType.MAIN_STATUS, mstate=self.mstate)

  def handle_MAFIA_STATUS(self, mstate, **kwargs):
    mstate.mresp(MRespType.MAFIA_STATUS, mstate=mstate)

  def handle_DM_STATUS(self, **kwargs):
    self.mstate.mresp(MRespType.DM_STATUS, mstate=self.mstate, player_id=kwargs['player_id'])

  def handle_MAIN_LEAVE(self, mstate, **kwargs):
    pass

  def handle_TIMER(self, mstate, **kwargs):
    try:
      player_id = kwargs['player_id']
      assert player_id in mstate.players, "[{player_id}] can't timer, they aren't playing"
      assert not player_id in mstate.timerers, "[{player_id}] has already timered"
    except AssertionError as e:
      msg = str(e)
      mstate.mresp(MRespType.TIMER_ERROR, reason=msg.format(**kwargs))

    # check timer status for mstate
    n_timers = len(mstate.timerers)
    n_players = len(mstate.players)

    def timer_end(_):
      mstate.timer()
    
    def reminder(n):
      mstate.mresp(MRespType.TIMER_REMINDER, minutes=n//60)

    if n_timers == 0:
      # start timer
      mstate.timer = MTimer(TIMER_LEN*n_players, [timer_end,0], [(reminder, 1*60),(reminder, 5*60),(reminder, 10*60),(reminder, 30*60),(reminder, 60*60)])
      mstate.mresp(MRespType.START_TIMER, player_id=player_id, time=mstate.timer.getTime() )
    else:
      mstate.timer.addTime(-TIMER_LEN)
      mstate.mresp(MRespType.REMOVE_TIME, player_id=player_id, time=mstate.timer.getTime())

    mstate.timerers.add(player_id)

  def handle_UNTIMER(self, mstate, **kwargs):
    try:
      player_id = kwargs['player_id']
      assert player_id in mstate.players, "[{player_id}] can't untimer, they aren't playing"
      assert player_id in mstate.timerers, "[{player_id}] hasn't already timered, ergo they can't untimer"
    except AssertionError as e:
      msg = str(e)
      mstate.mresp(MRespType.UNTIMER_ERROR, reason=msg.format(**kwargs))

    # check timer status for mstate
    n_timers = len(mstate.timerers)
    n_players = len(mstate.players)

    mstate.timerers.remove(player_id)

    if n_timers == 1:
      # cancel timer
      mstate.timer.cancel()
      mstate.mresp(MRespType.CANCEL_TIMER, player_id=player_id)
    else:
      mstate.mresp(MRespType.ADD_TIME, player_id=player_id, time=mstate.timer.getTime()-TIMER_LEN)
      mstate.timer.addTime(TIMER_LEN)
    
