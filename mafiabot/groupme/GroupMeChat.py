
from typing import Tuple, Dict
import time

from mafiabot import MChat, MDM, CastError

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

CAST_DELAY = .1

class GroupMeChat(MChat):

  def __init__(self, group_id, name_reference:MChat = None):
    self.id = group_id
    self.group = client.groups.get(group_id)
    self.setNameReference(name_reference)
    # Get member names
    self.names = {}
    for member in self.group.members:
      self.names[member.user_id] = member.nickname
      # self.names[member.user_id] = member.name

  def destroy(self):
    self.group.destroy()

  @staticmethod
  def new(name):
    g = client.groups.create(name)
    time.sleep(.5)
    bot = g.create_bot("Mafia Bot", callback_url = "http://70.180.16.29:1121/", dm_notification=False)
    return GroupMeChat(g.id)

  def __format(self, msg): # Default format if no name_reference
    for id,name in self.names.items():
      msg = msg.replace("[{}]".format(id),name)
    return msg

  def getName(self, user_id):
    if user_id in self.names:
      return self.names[user_id]
    else:
      return "(Couldn't get name)"

  def remove(self, user_id):
    if user_id == MODERATOR:
      return
    for member in self.group.members:
      if member.user_id == user_id:
        del self.names[member.user_id]
        member.remove()

  def refill(self, users):
    """Remove all members except those that will be added, then add"""
    self.group.update()
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
      self.names[user_id] = name
      user_submission.append({'user_id':user_id,'nickname':name})
    try:
      self.group.memberships.add_multiple(*user_submission)
      self.group.update()
    except Exception as e:
      raise CastError(e)

  def cast(self, msg:str):
    try:
      formatted_msg = self.format(msg)
      m_id = self.group.post(formatted_msg).id
      time.sleep(CAST_DELAY)
      print(m_id)
      return m_id
    except Exception as e:
      raise CastError(e)
    return "-1"

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

  def getAcks(self, message_id):
    msg_id = str(int(message_id)-1) # Subtract 1 so that our message shows up
    for msg in self.group.messages.list_after(msg_id):
      if msg.id == message_id:
        return msg.favorited_by
    return []

# Have all DMs in one object?
class GroupMeDM(MDM):

  def __init__(self, chat=None):
    self.client = client
    self.dms = {}
    if chat == None:
      self.format = lambda x:x
    else:
      self.format = chat.format

  def send(self, msg, user_id):
    message = self.format(msg)
    if not user_id in self.dms:
      self.dms[user_id] = DirectMessages(self.client.session, user_id)
    try:
      m_id = self.dms[user_id].create(message).id
      time.sleep(CAST_DELAY)
    except Exception as e:
      raise CastError(e)
    return m_id
