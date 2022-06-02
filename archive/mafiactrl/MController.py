
from typing import List, Dict, Set
from collections import deque
import json
from . import MGame, MTimer, MLobby
from ..resp_lib import get_resp
from ..mafiastate import MRules, MRoleGen, EndGameException
from ..chatinterface import MChat, MDM, MServer, MCmd

MIN_PLAYERS = 3
TIMER_MINUTES = 10

class MController:
  """
  Starts a server, tracks lobbies and games, routes incomings commands to lobbies/games
  This class can be subclassed to define the type of Chat, DM, Server, Game, etc.
  """
  
  MChatType = MChat
  MDMType = MDM
  MServerType = MServer
  MGameType = MGame
  MLobbyType = MLobby
  MTimerType = MTimer

  def __init__(self, lobby_ids):
    
    self.lobbies = dict( [(l_id,self.MLobbyType(self,l_id)) for l_id in lobby_ids] )
    self.dms = self.MDMType()
    self.rules = MRules()
    self.games:Dict[int,MGame] = {}
    self.focusedGames = {}

    # Check for active games?

  def run(self):
    server = self.MServerType(self.handle_chat, self.handle_dm)
    server.run()
    
  # callback for Server
  def handle_chat(self, group_id, sender_id, cmd:MCmd, **kwargs):
    cmd = MCmd(cmd)
    # First check games
    for g in self.games.values():
      try:
        if g.handle_chat(group_id, sender_id, cmd, **kwargs):
          return True
      except EndGameException as ege:
        # Game ended, find lobby associated with it.
        ## WHAT ABOUT TIMER ENDS???
        # Ok we actually need to pass a callback into MGame?
        # On game end, it is called and does this. When a game is created, set callback
        for l in self.lobbies.values():
          if g.id in l.game_ids:
            l.handle_end(g.id, ege.msg)

          
    
    for l in self.lobbies.values():
      if l.handle_chat(group_id, sender_id, cmd, **kwargs):
        return True
    
    return False

  def handle_dm(self, sender_id, cmd, **kwargs):
    cmd = MCmd(cmd)
    # check for this player's game?
    if cmd.is_game_dm():
      if sender_id in self.focusedGames:
        game = self.focusedGames[sender_id][0]
        if not game == None:
          game.handle_dm(sender_id, cmd, **kwargs)
          return True
    # Resolve any other dms we can?
    if cmd == MCmd.FOCUS:
      # Code to change the focused game of this player
      if sender_id in self.focusedGames:
        g_list = self.focusedGames[sender_id]
        g = g_list.popLeft()
        g_list.append(g)
        g_id = self.games[g_list[0]]
        self.dms.send("Focusing on Game {}".format(g_id), sender_id)

  def start_game(self, users, rules, lobby=None):
    g = self.MGameType.new()
    g.end_game = self.end_game
    g.destroy_callback = self.destroy_callback
    self.games[g.id] = g
    for user_id in users:
      if not user_id in self.focusedGames:
        self.focusedGames[user_id] = deque()
      self.focusedGames[user_id].appendleft(g.id)
    g.start(users, MRoleGen.roleGen)
    return g.id

  def end_game(self, g_id, msg):
    for l in self.lobbies.values():
      if g_id in l.game_ids:
        l.handle_end(g_id, msg)
    
  def destroy_callback(self, g_id):
    if g_id in self.games:
      del self.games[g_id]

  def watch(self, sender_id, g_id):
    game = self.games[g_id]
    game.main_chat.add(sender_id)
    self.dms.send(sender_id, get_resp("WATCH",g_id=g_id))

  def save(self):
    # save lobbies
    # save games
    pass

  def load(self):
    # load lobbies
    # load games
    pass