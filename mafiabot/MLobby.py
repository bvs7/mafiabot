
from .MInfo import *
from .MRules import MRules
from .MGame import MGame

class MLobby:

  def __init__(self, ctrl, group_id, MChatType, dms):
    self.MChatType = MChatType
    self.lobbyChat = MChatType(group_id)
    self.dms = dms
    self.ctrl = ctrl

    self.in_list = []
    self.rules = MRules()

    self.games = []

  def handle(self, sender_id, command, text, data):
    if command == IN_CMD:
      if not sender_id in self.in_list:
        self.in_list.append(sender_id)
      msg = "In List:\n" # TODO: generalize text
      msg += "\n".join(["[{}]".format(i) for i in self.in_list])
      self.lobbyChat.cast(msg)
    if command == OUT_CMD:
      if sender_id in self.in_list:
        self.in_list.remove(sender_id)
        msg = "Removed [{}]".format(sender_id) # TODO: generalize text
      else:
        msg = "You weren't IN"
      self.lobbyChat.cast(msg)
    if command == START_CMD: # TODO: Simplify start
      
      users = {}
      for user_id in self.in_list:
        users[user_id] = self.lobbyChat.getName(user_id)

      def end_callback(game, e):
        self.ctrl.games.remove(game)
        self.lobbyChat.cast("Game ended: {}".format(e))
        game.end()

      game = MGame(self.MChatType, self.dms, self.rules, end_callback, users)
      self.games.append(game)
      
      for user in users:
        self.ctrl.activeGame[user] = game
      self.ctrl.games.append(game)
      self.in_list = []

    if command == HELP_CMD:
      msg = "Help Test"
      self.lobbyChat.cast(msg)

  def group_id(self):
    return self.lobbyChat.id
