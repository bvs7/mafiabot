import requests
import json

import mafiabot

dest_url = "http://70.180.16.29:1121"

class TestChat(mafiabot.MChat):
  pass

test_lobby_id = "30021302"

def lobby_in(lobby_id, user_id):

  data = {'group_id':lobby_id, 'text':'/in', 'sender_id':user_id}
  post = {'data':json.dumps(data).encode('ascii')}
  
  requests.post(dest_url, post)