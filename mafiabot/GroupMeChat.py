
from typing import Tuple, Dictionary
import time

from .MChat import MChat, MDM, CastError

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

  def __init__(self, group_id):
    self.id = group_id
    self.group = client.groups.get(group_id)
    # Get member names
    self.names = {}
    for member in self.group.members:
      self.names[member.user_id] = member.nickname
      # self.names[member.user_id] = member.name

  def refill(self, users):
    """Remove all members except those that will be added, then add"""
    for member in self.group.members:
      if not member.user_id in users:
        self.group.memberships.remove(member.user_id)
      else:
        del users[member.user_id]
    self.add(users)

"""Each given user must be a dictionary containing a nickname and 
either an email, phone number, or user_id. """
  def add(self, users : Dictionary[str,str]):
    user_submission = []
    for user_id, name in users.items():
      user_submission.append({'user_id':user_id,'nickname':name})
    try:
      self.group.memberships.add_multiple(*user_submission)
    except Exception as e:
      raise CastError(e)

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
