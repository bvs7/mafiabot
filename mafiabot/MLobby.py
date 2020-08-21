import threading
import time

from .MInfo import *
from .MRules import MRules, RULE_BOOK
from .MGame import MGame
from .MTimer import MTimer

MIN_PLAYERS = 3
TIMER_MINUTES = 10

class MLobby:

  def __init__(self, ctrl, group_id, MChatType, dms, roleGen, MTimerType):
    self.MChatType = MChatType
    self.lobbyChat = MChatType(group_id)
    self.dms = dms
    self.ctrl = ctrl
    self.roleGen = roleGen
    self.MTimerType = MTimerType

    def lobby_cast(msg):
      return self.lobbyChat.cast(self.lobbyChat.format(msg))

    self.lobby_cast = lobby_cast

    self.in_list = {}
    self.start_timer = None
    self.start_msg_id = None
    self.rules = MRules()

    self.games = []

  def handle(self, group_id, sender_id, command, text, data):
    if command in GAME_MAIN_COMMANDS:
      for game in self.games:
        if game.active() and group_id == game.main_id():
          return game.handle_main(sender_id, command, text, data)
    if command in GAME_MAFIA_COMMANDS:
      for game in self.games:
        if game.active() and group_id == game.mafia_id():
          return game.handle_mafia(sender_id, command, text, data)
    if command in LOBBY_COMMANDS:
      if self.group_id() == group_id:
        if self.handle_lobby(sender_id, command, text, data):
          return True

  def handle_lobby(self, sender_id, command, text, data):

    if command == IN_CMD:
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
      self.lobby_cast(self.lobbyChat.format(msg))
      return True

    if command == OUT_CMD:
      for (in_p,min_p) in self.in_list.items():
        if sender_id == in_p:
          break
      else:
        msg = "You weren't /in"
        self.lobby_cast(msg)
        return True
      del self.in_list[sender_id]
      msg = "Removed [{}]".format(sender_id) # TODO: generalize text
      self.lobby_cast(msg)
      return True

    if command == START_CMD:
      words = text.split()
      min_players = MIN_PLAYERS
      timer_minutes = TIMER_MINUTES
      if len(words) >= 3:
        try:
          min_players = int(words[2])
        except ValueError:
          self.lobby_cast("Couldn't understand min player parameter: {}".format(words[2]))
      if len(words) >= 2:
        try:
          timer_minutes = int(words[1])
        except ValueError:
          self.lobby_cast("Couldn't understand minutes parameter: {}".format(words[1]))

      # Send start msg and record message id
      msg = "Game will start in {} minute{} if at least {} players are in. Like this message to join!".format(
        timer_minutes, 's' if timer_minutes!=1 else '', min_players)
      self.start_msg_id = self.lobby_cast(msg)
      self.start_min_players = min_players
      self.start_timer = self.MTimerType(timer_minutes*60, {0:[self.try_start_game]})

    if command == WATCH_CMD:
      if len(self.games) == 1:
        name = self.lobbyChat.getName(sender_id)
        self.games[0].main_chat.add({sender_id:name})
      elif len(self.games) == 0:
        self.lobby_cast("Failed to watch, no games")
      else:
        self.lobby_cast("Failed to watch, can't tell which game...")

    if command == RULE_CMD:
      msg = ""
      words = text.split()
      if len(words) == 1:
        msg = self.rules.describe(has_expl=False)
      elif words[1] in RULE_BOOK:
        rule = words[1]
        msg = "{}:\n".format(rule)
        msg += self.rules.explRule(rule, self.rules[rule])
      elif words[1] == "long":
        msg = self.rules.describe(has_expl=True)
    
      self.lobby_cast(msg)

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

    def end_game_callback(game, e):
      self.ctrl.end_game_callback(game.state.id, game.main_chat.id, game.mafia_chat.id)
      msg = "Game {} ended: {}\n".format(game.state.id, e)
      msg += game.main_chat.format(game.state.start_roles)
      self.lobby_cast(msg)
      game.main_cast(msg)
      self.start_timer = None

    if len(users) > MIN_PLAYERS:
      self.lobby_cast("Starting game")
      game = MGame(self.MChatType, self.dms, self.rules, end_game_callback, users, self.roleGen)
      self.games.append(game)

      for user in users:
        if not user in self.ctrl.activeGame:
          self.ctrl.activeGame[user] = []
        self.ctrl.activeGame[user].append(game)
      self.in_list = {}
    else:
      self.lobby_cast("Could not start a game")

  def group_id(self):
    return self.lobbyChat.id
