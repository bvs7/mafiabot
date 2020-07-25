# Recieve slightly processed request from Server, route to the correct MState/lobby, respond, etc.

from typing import List

from MInfo import *
from .MGame import MGame
from .MRules import MRules
from .GroupMeChat import GroupMeChat, GroupMeDM

# TODO: multiple lobbies!, lobby instances
# TODO: getLobbyChats, getGameChats

class MController:

  # init

  def __init__(self, MChatType, MDMType, MServerType):
    self.MChatType = MChatType
    self.MServerType = MServerType

    self.lobbyChat = getLobbyChats(MChatType)
    self.gameChats = getGameChats(MChatType)
    self.games : List[MGame] = []
    self.dms = getDMs(MDMType)

    self.rules = MRules()

    self.activeGame = {} # Maps player id to their active game

    self.ins = []

    server = MServerType()
    serverThread = treading.Thread(target=server.serve_forever)
    serverThread.start()
    
  # callback for Server
  def handle_chat(self,  group_id, sender_id, command, text, data):
    if command in GAME_MAIN_COMMANDS:
      for game in self.games:
        if game.active() and group_id == game.main_id():
          game.handle_main(sender_id, command, text, data)
    if command in GAME_MAFIA_COMMANDS:
      for game in self.games:
        if group_id == game.mafia_id():
          game.handle_mafia(sender_id, command, text, data)
    if command in LOBBY_COMMANDS:
      self.handle_lobby(self, sender_id, command, text, data)

  def handle_lobby(self, sender_id, command, text, data):
    if command == IN_CMD:
      if not sender_id in self.ins:
        self.ins.append(sender_id)
      msg = "In List:\n"
      msg += "\n".join(["[{}]".format(i) for i in self.ins)
      self.lobbyChat.cast(msg)
    if command == OUT_CMD:
      self.ins.remove(sender_id)
      msg = "Removed [{}]".format(sender_id)
      self.lobbyChat.cast(msg)
    if command == START_CMD:
      game = MGame(self.gameChats[0], self.gameChats[1], self.dms, self.rules)
      self.games.append(game)
      users = []
      for i in self.ins:
        user = {'nickname' : "Tesing", 'user_id' : i}
        users.append(user)
      game.main.add(users)
      game.handle_start(self.ins)

  def handle_chat(self, sender_id, command, text, data):
    # check for this player's game?
    if command in GAME_DM_COMMANDS:
      if sender_id in self.activeGame:
        game = self.activeGame[sender_id]
        if not game == None:
          self.handle_game_dm(game, sender_id, command, text, data)
    # TODO:


if __name__ == '__main__':
  ctrl = MController(GroupMeChat, GroupMeDM, GroupMeServer)

  while True:
    pass