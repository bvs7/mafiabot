import threading
import time

from .MInfo import *
from .MRules import MRules
from .MGame import MGame

class MLobby:

  def __init__(self, ctrl, group_id, MChatType, dms):
    self.MChatType = MChatType
    self.lobbyChat = MChatType(group_id)
    self.dms = dms
    self.ctrl = ctrl

    def lobby_cast(msg):
      self.lobbyChat.cast(self.lobbyChat.format(msg))

    self.lobby_cast = lobby_cast

    self.in_list = []
    self.rules = MRules()

    self.games = []

  def handle(self, sender_id, command, text, data):
    if command == IN_CMD:
      if not sender_id in self.in_list:
        self.in_list.append(sender_id)
      msg = "In List:\n" # TODO: generalize text
      msg += "\n".join(["[{}]".format(i) for i in self.in_list])
      self.lobby_cast(msg)
    if command == OUT_CMD:
      if sender_id in self.in_list:
        self.in_list.remove(sender_id)
        msg = "Removed [{}]".format(sender_id) # TODO: generalize text
      else:
        msg = "You weren't IN"
      self.lobby_cast(msg)
    if command == START_CMD: # TODO: Simplify start
      users = {}
      for user_id in self.in_list:
        users[user_id] = self.lobbyChat.getName(user_id)

      def end_game(game):
        time.sleep(300)
        game.end()

      def end_callback(game, e):
        self.ctrl.games.remove(game)
        for user in self.ctrl.activeGame:
          self.ctrl.activeGame = None
        msg = "Game {} ended: {}\n".format(game.state.id, e)
        msg += dispStartRoles(game.state.start_roles)
        self.lobby_cast(msg)
        thread = threading.Thread(target=end_game, args=(game,))
        thread.start()

      game = MGame(self.MChatType, self.dms, self.rules, end_callback, users)
      self.games.append(game)

      for user in users:
        self.ctrl.activeGame[user] = game
      self.ctrl.games.append(game)
      self.in_list = []

    if command == HELP_CMD:
      msg = "Help Test"
      self.lobby_cast(msg)

    if command == WATCH_CMD:
      if len(self.games) == 1:
        name = self.lobbyChat.getName(sender_id)
        self.games[0].main_chat.add({sender_id:name})
      elif len(self.games) == 0:
        self.lobby_cast("Failed to watch, no games")
      else:
        self.lobby_cast("Failed to watch, can't tell which game...")

  def group_id(self):
    return self.lobbyChat.id
