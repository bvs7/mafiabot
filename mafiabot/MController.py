# Recieve slightly processed request from Server, route to the correct MState/lobby, respond, etc.

from typing import List, Dict, Set
from collections import deque
import json

import mafiabot

from .MInfo import *
from .MGame import MGame
from .MRules import MRules
from .MRoleGen import MRoleGen
from .MTimer import MTimer
from .MChat import MChat, MDM
from .MServer import MServer
from .MPlayer import MPlayer
from .MSave import mafia_hook, MSaveEncoder

MIN_PLAYERS = 3
TIMER_MINUTES = 10

class MController:
  
  MChatType = MChat
  MDMType = MDM
  MServerType = MServer
  MGameType = MGame

  def __init__(self, lobby_id):
    
    self.lobbyChat = self.MChatType(lobby_id)
    self.dms = self.MDMType(self.lobbyChat)
    self.rules = MRules()
    self.games:Dict[int,MGame] = {}
    self.focusedGames = {}
    self.in_list = {} # maps id to min_p

    # Check for active games?
    try:
      f = open("./data/lobbies/{}".format(lobby_id))
      self.parse(f)
      f.close()
    except FileNotFoundError:
      pass

  def run(self):
    server = self.MServerType(self.handle_chat, self.handle_dm)
    server.run()
    
  # callback for Server
  def handle_chat(self, group_id, sender_id, cmd:MCmd, **kwargs):
    cmd = MCmd(cmd)
    # First check games
    for g_id,g in self.games.items():
      if g.handle_chat(group_id, sender_id, cmd, **kwargs):
        return True
    
    if group_id == self.lobbyChat.id:
      self.handle_lobby(group_id, sender_id, cmd, **kwargs)

  def handle_lobby(self, group_id, sender_id, cmd, **kwargs):

    text = kwargs["text"]
    if cmd == MCmd.IN:
      # Get param /in [min]
      words = text.split()
      if len(words) > 1:
        try:
          min_players = int(words[1])
        except ValueError:
          min_players = MIN_PLAYERS
      else:
        min_players = MIN_PLAYERS
      
      self.in_list[sender_id] = min_players

      msg = "[{}] ready to join a game of at least {} players. ".format(sender_id, min_players)
      msg += "{} ready to play.".format(len(self.in_list))
      self.lobbyChat.cast(msg)
      return True

    if cmd == MCmd.OUT:
      for in_p in self.in_list:
        if sender_id == in_p:
          break
      else:
        msg = "You weren't /in"
        self.lobbyChat.cast(msg)
        return True
      del self.in_list[sender_id]
      msg = "Removed [{}]".format(sender_id) # TODO: generalize text
      self.lobbyChat.cast(msg)
      return True

    if cmd == MCmd.START:
      words = text.split()
      min_players = MIN_PLAYERS
      timer_minutes = TIMER_MINUTES
      if len(words) >= 3:
        try:
          min_players = int(words[2])
        except ValueError:
          self.lobbyChat.cast("Couldn't understand min player parameter: {}".format(words[2]))
      if len(words) >= 2:
        try:
          timer_minutes = int(words[1])
        except ValueError:
          self.lobbyChat.cast("Couldn't understand minutes parameter: {}".format(words[1]))

      # Send start msg and record message id
      msg = "Game will start in {} minute{} if at least {} players are in. Like this message to join!".format(
        timer_minutes, 's' if timer_minutes!=1 else '', min_players)
      self.start_msg_id = self.lobbyChat.cast(msg)
      self.start_min_players = min_players
      self.start_timer = MTimer(timer_minutes*5, {0:[self.try_start_game]})

    if cmd == MCmd.WATCH:
      if len(self.games) == 1:
        name = self.lobbyChat.getName(sender_id)
        self.games[0].main_chat.add({sender_id:name})
      elif len(self.games) == 0:
        self.lobbyChat.cast("Failed to watch, no games")
      else:
        self.lobbyChat.cast("Failed to watch, can't tell which game...")

    if cmd == MCmd.STATUS:
      self.lobbyChat.cast("Status not implemented yet")
    
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

  def start_game(self, users, rules):
    g = self.MGameType.new(rules)
    self.games[g.id] = g
    for user_id in users:
      if not user_id in self.focusedGames:
        self.focusedGames[user_id] = deque()
      self.focusedGames[user_id].appendleft(g.id)
    g.start(users, MRoleGen.roleGen)

  def try_start_game(self):

    ack_in_list = [(p_id, self.start_min_players) for p_id in self.lobbyChat.getAcks(self.start_msg_id)]

    for (p_id, min_p) in ack_in_list:
      if not p_id in self.in_list:
        self.in_list[p_id] = min_p

    # Sort in_list from largest to smallest min_players
    in_list = self.in_list.items()
    in_list = sorted(in_list, key=lambda x: x[1])
    in_list.reverse()
    # While not done, test for a game, if not, remove largest min_players
    done = False
    users = {}
    while len(in_list) > 0:
      if len(in_list) >= in_list[0][1]:
        for user_id,_ in in_list:
          users[user_id] = self.lobbyChat.getName(user_id)
        break
      else:
        in_list = in_list[1:]

    if len(users) >= MIN_PLAYERS:
      self.lobbyChat.cast("Starting game")
      self.start_game(users, MRules(self.rules))
      self.in_list = {}
    else:
      self.lobbyChat.cast("Could not start a game")

  def destroy_game(self,game, id):
    if id in self.games:
      del self.games[id]

    for p_id in self.focusedGames:
      game_list = self.focusedGames[p_id]
      if id in game_list:
        game_list.remove(id)

    for p_id in list(self.focusedGames.keys()):
      if len(self.focusedGames[p_id]) == 0:
        del self.focusedGames[p_id]
  
  def load_game(self, g_id):
    f = open("./data/games/game_{}.maf".format(g_id), 'r')
    g = self.MGameType.load(f)
    self.games[g.id] = g
    f.close()

  def save(self):
    f = open("./data/lobbies/{}".format(self.lobbyChat.id), 'w')
    f.write("Rules: {}".format(json.dumps(self.rules, cls=MSaveEncoder)))
    f.write("Games: {}".format(json.dumps(list(self.games.keys()))))
    f.write("FocusedGames: {}".format(json.dumps(self.focusedGames)))
    f.write("InList: {}".format(json.dumps(self.in_list)))

  def parse(self,f):
    lines = f.readlines()
    for line in lines:
      if line.startswith('Rules:'):
        rule_str = line[len("Rules: "):]
        self.rules = json.loads(rule_str, object_hook=mafia_hook)
      elif line.startswith('Games: '):
        games_str = line[len("Games: "):]
        game_ids = json.loads(games_str)
        for game_id in game_ids:
          self.load_game(game_id)
      elif line.startswith("FocusedGames: "):
        fg_str = line[len("FocusedGames: "):]
        self.focusedGames = json.loads(fg_str)
      elif line.startswith("InList: "):
        inl_str = line[len("InList: "):]
        self.in_list = json.loads(inl_str)