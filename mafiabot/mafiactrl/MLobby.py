import threading
import time
from typing import List, Deque
from collections import deque

from . import MTimer

from ..mafiastate import MRules
from ..chatinterface import MCmd
from .. import resp_lib

# Eh for now have controller and lobby be the same

class MLobby:

  DEFAULT_MIN_PLAYERS = 3
  DEFAULT_TIMER_MINUTES = 10

  def __init__(self, ctrl, lobby_id):
    self.ctrl = ctrl
    self.chat = ctrl.MChatType(lobby_id)
    self.game_ids : Deque[int] = deque()

    self.in_list = {}
    self.start_timer = None
    self.start_msg_id = None
    self.rules = MRules()

  def handle_chat(self, group_id, sender_id, cmd, **kwargs):

    if cmd == MCmd.IN:
      self.handle_in(sender_id, **kwargs)

    if cmd == MCmd.OUT:
      # remove sender from in_list
      pass

    if cmd == MCmd.START:
      # Start game or start timer.
      # call self.ctrl.start_game when ready, put resulting id in self.game_ids
      pass

    if cmd == MCmd.WATCH:
      # Check game_ids for desired target game
      # Add sender to watchable game
      pass

    if cmd == MCmd.STATUS:
      # Check game_ids for desired target game
      # Return status of desired game
      pass

    return False

  def handle_in(self, sender_id, min_p):
    self.in_list[sender_id] = min_p
    self.chat.castresp("IN",**locals())
    return True
    
  def handle_out(self, sender_id):
    if sender_id in self.in_list:
      del self.in_list[sender_id]
      self.chat.castresp("OUT",**locals())
    else:
      self.chat.castresp("OUT_NOT_IN",**locals())
    return True

  def handle_start(self, minutes, min_p):
    self.start_msg_id = self.chat.castresp("START_TIMER",**locals())
    self.start_min_players = min_p
    self.start_timer = self.ctrl.MTimerType(minutes*60, {0:[self.try_start_game]})

  def handle_watch(self, sender_id, g_id=None):
    if len(self.game_ids) == 0:
      self.chat.castresp("WATCH_NO_GAMES")
      return True
    if not g_id:
      g_id = self.game_ids[0]
    self.ctrl.watch(sender_id, g_id)

  def try_start_game(self):

    ack_in_list = [(p_id, self.start_min_players) for \
      p_id in self.chat.getAcks(self.start_msg_id)]

    # Merge dictionaries, sort by largest to smallest min_p
    full_in_list = ack_in_list | self.in_list
    full_in_list = full_in_list.items()
    full_in_list = sorted(full_in_list, key=lambda x: -x[1])
    
    users = {}
    while len(full_in_list) > 0:
      # Check if player with largest game wants in
      if len(full_in_list) >= full_in_list[0][1]:
        for user_id,_ in full_in_list:
          users[user_id] = self.chat.getName(user_id)
        break
      else:
        full_in_list = full_in_list[1:]
    
    if len(users) >= self.DEFAULT_MIN_PLAYERS:
      player_list = "\n".join(["[{}]".format(u) for u in users])
      self.chat.castresp("START_GAME", **locals())
      g_id = self.ctrl.start_game(users, MRules(self.rules), lobby=self)
      self.game_ids.appendleft(g_id)
    else:
      self.chat.castresp("FAILED_START_GAME")


    

