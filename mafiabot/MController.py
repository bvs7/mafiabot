# Recieve slightly processed request from Server, route to the correct MState/lobby, respond, etc.

from typing import List

from .MInfo import *
from .MGame import MGame
from .MRules import MRules
from .MLobby import MLobby

def getLobbies(ctrl, MChatType, dms):
  lobbies = []
  lobbies.append(MLobby(ctrl, '30021302', MChatType, dms))
  return lobbies
  
def getGames(MChatType):
  return []

def getDMs(MDMType):
  return MDMType()

# TODO: multiple lobbies!, lobby instances
# TODO: getLobbyChats, getGameChats. Flesh out

# TODO: Proper RoleGen!
# TODO: Rule edits and checking
# TODO: Save state and refreshing games
# TODO: Database Recording and stats

class MController:

  # init

  def __init__(self, MChatType, MDMType, MServerType, debug=None):
    self.MChatType = MChatType
    self.MServerType = MServerType

    self.games = getGames(MChatType)
    self.dms = getDMs(MDMType)
    
    self.lobbies = getLobbies(self, MChatType, self.dms)

    self.rules = MRules()

    self.activeGame = {} # Maps player id to their active game

    self.ins = []

    if not debug == None:
      debug(self)

    server = MServerType(self.handle_chat, self.handle_dm)
    
  # callback for Server
  def handle_chat(self,  group_id, sender_id, command, text, data):
    if command in GAME_MAIN_COMMANDS:
      for game in self.games:
        if game.active() and group_id == game.main_id():
          game.handle_main(sender_id, command, text, data)
    if command in GAME_MAFIA_COMMANDS:
      for game in self.games:
        if game.active() and group_id == game.mafia_id():
          game.handle_mafia(sender_id, command, text, data)
    if command in LOBBY_COMMANDS:
      for lobby in self.lobbies:
        if lobby.group_id() == group_id:
          lobby.handle(sender_id, command, text, data)

  def handle_dm(self, sender_id, command, text, data):
    # check for this player's game?
    if command in GAME_DM_COMMANDS:
      if sender_id in self.activeGame:
        game = self.activeGame[sender_id]
        if not game == None:
          game.handle_dm(sender_id, command, text, data)