from mafiabot.groupme import GroupMeServer
from mafiabot.test import TestRequests
import mafiabot
from collections import deque
import time
import threading
import unittest

def create_test_handle_chat():
  messages = deque()
  def handle_chat(group_id, sender_id, command, **kwargs):
    kwargs['group_id'] = group_id
    kwargs['sender_id'] = sender_id
    kwargs['command'] = command
    try:
      message = messages.popleft()
      for key in message:
        if not kwargs[key] == message[key]:
          raise ValueError(key, kwargs[key], message[key])
      print("handle_chat success: {}".format(message))
      return True
    except Exception as e:
      print("handle_chat failed: {}".format(e))
      raise e
    return False
  
  def add_chat(message):
    messages.append(message)

  return handle_chat, add_chat

def create_test_handle_dm():
  messages = deque()
  def handle_dm(sender_id, command, **kwargs):
    kwargs['sender_id'] = sender_id
    kwargs['command'] = command
    try:
      message = messages.popleft()
      for key in message:
        if not kwargs[key] == message[key]:
          raise ValueError(key, kwargs[key], message[key])
      print("handle_dm success: {}".format(message))
      return True
    except Exception as e:
      print("handle_dm failed: {}".format(e))
      raise e
    return False
  
  def add_dm(message):
    messages.append(message)

  return handle_dm, add_dm

class TestServer(unittest.TestCase):

  def setUp(self):
    handle_chat, self.add_chat = create_test_handle_chat()
    handle_dm, self.add_dm = create_test_handle_dm()

    self.gms = GroupMeServer(handle_chat, handle_dm)
    self.gms.app.testing = True
    self.app = self.gms.app.test_client()

  def tearDown(self):
    del self.gms
    del self.app

  def test_my_dms(self):
    self.add_dm({"text":"/status","sender_id":"0","command":mafiabot.MCmd.STATUS})
    self.app.post("/dm", data=TestRequests.dm_data(text="/status"))


  def test_chat(self):
    self.add_chat({"group_id":"0","text":"/status","sender_id":"0", "command":mafiabot.MCmd.STATUS})
    self.app.post("/", data=TestRequests.cast_data(text="/status"))



