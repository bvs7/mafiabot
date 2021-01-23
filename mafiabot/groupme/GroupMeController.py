import mafiabot

from .GroupMeChat import GroupMeChat, GroupMeDM
from .GroupMeGame import GroupMeGame
from .GroupMeServer import GroupMeServer


class GroupMeController(mafiabot.MController):

  MChatType = GroupMeChat
  MDMType = GroupMeDM
  MGameType = GroupMeGame
  MServerType = GroupMeServer

class TestGroupMeController(mafiabot.MController):

  MChatType = mafiabot.MChat
  MDMType = mafiabot.MDM
  MGameType = GroupMeGame
  MServerType = GroupMeServer

  def __init__(self, lobby_id):
    super().__init__(lobby_id)
    self.MGameType.MChatType = self.MChatType
    self.MGameType.MDMType = self.MDMType

  def run(self):
    server = self.MServerType(self.handle_chat, self.handle_dm)
    server.run(debug=True)

