
from . import MController

from ..chatinterface import MChat, MDM
from ..chatinterface.groupme import GroupMeChat, GroupMeDM, GroupMeServer, GroupMeGame


class GroupMeController(MController):

  MChatType = GroupMeChat
  MDMType = GroupMeDM
  MGameType = GroupMeGame
  MServerType = GroupMeServer

class TestGroupMeController(MController):

  MChatType = MChat
  MDMType = MDM
  MGameType = GroupMeGame
  MServerType = GroupMeServer

  def __init__(self, lobby_id, MChatType, MDMType):
    super().__init__(lobby_id)
    self.MGameType.MChatType = MChatType
    self.MGameType.MDMType = MDMType

  def run(self):
    server = self.MServerType(self.handle_chat, self.handle_dm)
    server.run(debug=True)

