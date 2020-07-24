
from .MChat import MChat, MDM, CastError
import time

GROUPY_KEYFILE = "../../.groupy.key"

try:
  import groupy
  from groupy.api.messages import Messages, DirectMessages
  tokenfile = open(GROUPY_KEYFILE, 'r')
  token = tokenfile.read().strip()
  tokenfile.close()

  client = groupy.Client.from_token(token)

CAST_DELAY = .5

class GroupMeChat(MChat):

  def __init__(self, group_id, client):
    # Create? no, just find
    self.group = client.groups.get(group_id)
    self.m = Messages(client.sesion, group_id)

  def cast(self, msg):
    try:
      m_id = self.m.create(msg).id
      time.sleep(CAST_DELAY)
    except Exception as e:
      raise CastError(e)

  def ack(self, message_id):
    try:
      messages = self.group.messages.list_all_after(str(int(message_id)-1))
      for message in messages:
        if message.id == message_id:
          m = message
          break
      m.like()
    except groupy.exceptions.GroupyError:
      return False
    return True

class GroupMeDM(MDM):

  def __init__(self, user_id, client):
    self.dm = DirectMessages(client.session, user_id)

  def send(self, msg):
    try:
      m_id = self.dm.create(msg).id
      time.sleep(CAST_DELAY)
    except Exception as e:
      raise CastError(e)
    return m_id
