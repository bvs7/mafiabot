
from MInfo import *
from .MRules import MRules

class MLobby:

  def __init__(self, group_id, MChatType):

    self.lobbyChat = MChatType(group_id)

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
    if command == START_CMD:
      game = MGame(self.gameChats[0], self.gameChats[1], self.dms, self.rules)
      self.games.append(game)
      users = {}
      for user_id in self.in_list():
        users[user_id] = self.chat.names[user_id]
      game.handle_start(self.in_list)
    if command == HELP_CMD:
      msg = "Help Test"
      self.lobbyChat.cast(msg)

  def group_id(self):
    return self.lobbyChat.id
