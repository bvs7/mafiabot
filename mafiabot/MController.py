# Recieve slightly processed request from Server, route to the correct MState/lobby, respond, etc.

from typing import List

from .MInfo import *
from .MGame import MGame
from .MRules import MRules, RULE_BOOK
from .MLobby import MLobby
from .MRoleGen import randomRoleGen
from .MTimer import MTimer

def getLobbies(ctrl, MChatType, dms):
  lobbies = []
  lobbies.append(MLobby(ctrl, '30021302', MChatType, dms))
  lobbies.append(MLobby(ctrl, '60988610', MChatType, dms))
  lobbies.append(MLobby(ctrl, '25833774', MChatType, dms))
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

  def __init__(self, MChatType, MDMType, MServerType, debug=None):
    self.MChatType = MChatType
    self.MServerType = MServerType
    self.dms = getDMs(MDMType)
    
    self.lobbies = getLobbies(self, MChatType, self.dms)
    self.orphaned_chats = {} # Dict of chat_id => timer

    # TODO: a set of rules for each dm/players
    self.rules = MRules()

    self.activeGame = {} # Maps player id to their active game (or a list of active games)

    self.rolegen_opine = {} # Maps player_id -> (roles,contracts)

    self.ins = []

    if not debug == None:
      debug(self)

    server = MServerType(self.handle_chat, self.handle_dm)
    
  # callback for Server
  def handle_chat(self,  group_id, sender_id, command, text, data):
    # Each lobby tries to handle
    for lobby in self.lobbies:
      if lobby.handle(group_id, sender_id, command, text, data):
        return True
    # handle default mafia bot actions
    if command == HELP_CMD:
      words = text.split()
      if len(words) > 0:
        topic = words[1]
        if topic in ROLE_EXPLAIN:
          resp = self.MChatType(group_id)
          resp.cast(ROLE_EXPLAIN[topic])

  def handle_dm(self, sender_id, command, text, data):
    # check for this player's game?
    if command in GAME_DM_COMMANDS:
      if sender_id in self.activeGame:
        try:
          game = self.activeGame[sender_id][0]
          if not game == None:
            game.handle_dm(sender_id, command, text, data)
            return True
        except (IndexError,KeyError):
          pass

    if command == 'rolegen':
      try:
        n = int(text.split()[1])
        temp_ids = [i for i in range(n)]
        ids,roles,contracts = randomRoleGen(temp_ids)
        role_disp = dispRoleFromDict(makeRoleDict(roles))
        contracts_disp = []
        for p_id in contracts:
          (role,charge,success) = contracts[p_id]
          contracts_disp.append("{}->{}".format(role,roles[charge]))
        msg = "Roles:\n{}\nContracts: {}\n(Respond /opine [1-5] to give opinion on this rolegen (5-best, 1-worst))".format(role_disp,contracts_disp)
        self.rolegen_opine[sender_id] = (roles,contracts)
        self.dms.send(msg, sender_id)
      except Exception as e:
        msg = "Sorry, I had an error: {}".format(e)
        self.dms.send(msg, sender_id)
    elif command == 'opine':
      try:
        opinion = int(text.split()[1])
        if not (opinion >= 1 and opinion <= 5):
          raise IndexError("Opinion should be from 1 to 5")
        if sender_id in self.rolegen_opine:
          roles, contracts = self.rolegen_opine[sender_id]
          with open("../data/rolegen_opinions", 'a') as rof:
            rof.write("{}\n{}\n{}".format(roles,contracts,opinion))
          msg = "Opinion noted, thank you!"
          del self.rolegen_opine[sender_id]
        else:
          msg = "Nothing to opine upon"
        self.dms.send(msg, sender_id)
      except Exception as e:
        msg = "Sorry, I had an error: {}".format(e)
        self.dms.send(msg, sender_id)
    elif command == RULE_CMD:
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
    
      self.dms.send(msg, sender_id)

  def end_game_callback(self, game_id, main_chat_id, mafia_chat_id):
    # Orphan chats
    def remove_main_orphan():
      del self.orphaned_chats[main_chat_id]

    def kill_main_chat():
      chat = self.MChatType(main_chat_id)
      chat.cast("Destroying Chat")
      chat.destroy()

    def remove_mafia_orphan():
      del self.orphaned_chats[mafia_chat_id]

    def kill_mafia_chat():
      chat = self.MChatType(mafia_chat_id)
      chat.cast("Destroying Chat")
      chat.destroy()
      
    self.orphaned_chats[main_chat_id] = MTimer(30*60, {0:[remove_main_orphan, kill_main_chat]})
    self.orphaned_chats[mafia_chat_id] = MTimer(30*60, {0:[remove_mafia_orphan, kill_mafia_chat]})
    
    for player in self.activeGame:
      games = self.activeGame[player]
      for game in games:
        if game_id == game.state.id:
          self.activeGame[player].remove(game)


    

