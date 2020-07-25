
from typing import Tuple, Dict
import time

from .MChat import MChat, MDM, CastError

GROUPME_KEYFILE = "../../.groupme.key"
MODERATOR = "43040067"

try:
  import groupy
  from groupy.api.messages import Messages, DirectMessages
  tokenfile = open(GROUPME_KEYFILE, 'r')
  token = tokenfile.read().strip()
  tokenfile.close()

  client = groupy.Client.from_token(token)
except Exception as e:
  print("Failed to import groupy: " + str(e))

CAST_DELAY = .5

class GroupMeChat(MChat):

  def __init__(self, group_id):
    self.id = group_id
    self.group = client.groups.get(group_id)
    # Get member names
    self.names = {}
    for member in self.group.members:
      self.names[member.user_id] = member.nickname
      # self.names[member.user_id] = member.name


  def remove(self, user_id):
    if user_id == MODERATOR:
      return
    for member in self.group.members:
      if member.user_id == user_id:
        member.remove()

  def refill(self, users):
    """Remove all members except those that will be added, then add"""
    self.group = self.group.update()
    member_ids = [mem.user_id for mem in self.group.members]
    for member_id in member_ids:
      if not member_id in users:
        self.remove(member_id)
      else:
        del users[member_id]
    self.add(users)

  def add(self, users : Dict[str,str]):
    if len(users) == 0:
      return
    user_submission = []
    for user_id, name in users.items():
      user_submission.append({'user_id':user_id,'nickname':name})
    try:
      self.group.memberships.add_multiple(*user_submission)
    except Exception as e:
      raise CastError(e)

  def cast(self, msg):
    try:
      m_id = self.group.post(msg).id
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

# Have all DMs in one object?
class GroupMeDM(MDM):

  def __init__(self, client):
    self.client = client
    self.dms = {}

  def send(self, msg, user_id):
    if not user_id in self.dms:
      self.dms[user_id] = DirectMessages(self.client, user_id)
    try:
      m_id = self.dm.create(msg).id
      time.sleep(CAST_DELAY)
    except Exception as e:
      raise CastError(e)
    return m_id