# Recieve slightly processed request from Server, route to the correct MState/lobby, respond, etc.

from typing import List

from MInfo import *
from .MGame import MGame
from .MRules import MRules
from .GroupMeChat import GroupMeChat, GroupMeDM
from .MLobby import MLobby

def getLobbies(MChatType):
  lobbies = []
  lobbies.append(MLobby('30021302', MChatType))
  return lobbies

# TODO: multiple lobbies!, lobby instances
# TODO: getLobbyChats, getGameChats

class MController:

  # init

  def __init__(self, MChatType, MDMType, MServerType):
    self.MChatType = MChatType
    self.MServerType = MServerType

    self.lobbies = getLobbies(MChatType)
    self.games = getGames(MChatType)
    self.dms = getDMs(MDMType)

    self.rules = MRules()

    self.activeGame = {} # Maps player id to their active game

    self.ins = []

    server = MServerType(self.handle_chat, self.handle_dm)
    
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
      for lobby in self.lobbies:
        if lobby.group_id() == group_id:
          lobby.handle(sender_id, command, text, data)

    # TODO: leave, help?

  def handle_dm(self, sender_id, command, text, data):
    # check for this player's game?
    if command in GAME_DM_COMMANDS:
      if sender_id in self.activeGame:
        game = self.activeGame[sender_id]
        if not game == None:
          self.handle_game_dm(game, sender_id, command, text, data)
    # TODO:
