import json

cast_address = "http://localhost:1121/"
dm_address = "http://localhost:1121/dm"

default_data = {
  'attachments' : [],
  'avatar_url': "AVATAR_URL",
  'created_at': 0,
  'group_id' : "0",
  'id' : "0",
  'name' : "___",
  'sender_id' : "0",
  'sender_type' : "user",
  'source_guid' : "GUID",
  'system': False,
  'text': "",
  'user_id':"0",
}

def cast_data(**kwargs):
  data = default_data.copy()
  data.update(kwargs)
  return json.dumps(data)

def dm_data(**kwargs):
  data = default_data.copy()
  data.update(kwargs)
  return json.dumps(data)