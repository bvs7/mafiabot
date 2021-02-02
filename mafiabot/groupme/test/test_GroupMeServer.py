import flask_unittest
import flask.globals
import json
from ... import test_util
from .. import GroupMeServer

def get_chat_data(**kwargs):
  data = {
    "attachments": [],
    "avatar_url": "",
    "created_at": 0,
    "group_id": "0",
    "id": "0",
    "name": "_",
    "sender_id": "",
    "sender_type": "user",
    "source_guid": "GUID",
    "system": False,
    "text": "",
    "user_id": "0"
  }
  for k,v in kwargs.items():
    data[k] = v
  return json.dumps(data)

def get_dm_data(**kwargs):
  data = {
    "attachments": [],
    "avatar_url": "",
    "created_at": 0,
    "id": "0",
    "name": "_",
    "sender_id": "",
    "sender_type": "user",
    "source_guid": "GUID",
    "system": False,
    "text": "",
    "user_id": "0"
  }
  for k,v in kwargs.items():
    data[k] = v
  return json.dumps(data) 

class Test_GroupMeServer(flask_unittest.ClientTestCase):

  gms = GroupMeServer(print,print)
  app = gms.app

  def setUp(self, client):
    self.handle_chat, self.add_chat = \
      test_util.create_handle_chat_tester(test_util.print_mode)
    self.handle_dm, self.add_dm = \
      test_util.create_handle_dm_tester(test_util.print_mode)
    self.gms.handle_chat = self.handle_chat
    self.gms.handle_dm = self.handle_dm

  def test_GroupMeServerSimple(self, client):
    self.add_chat("LOBBY", '1', 'status')
    client.post("/", data=get_chat_data(group_id="LOBBY", sender_id="1", text="/status"))

    self.add_dm("1","help")
    client.post('/dm', data=get_dm_data(sender_id="1", text="/help"))