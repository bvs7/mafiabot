from ..resp_lib import get_resp

class CastError(Exception):
  pass

class MChat:
  # TODO: Consider cast_resp, and even prep_resp which will load in a str and wait to cast it until next cast?

  def __init__(self, group_id, name_reference=None):
    print("MChat %s"%group_id, flush=True)
    self.id = group_id
    if name_reference == None:
      self.format = self.__format
    else:
      self.format = name_reference.format
    self.names = {}

  def setNameReference(self, name_reference):
    if name_reference == None:
      self.format = self.__format
    else:
      self.format = name_reference.format

  def destroy(self):
    print("DEL {}".format(self.id))

  @classmethod
  def new(cls, name, name_reference=None): # Just return id?
    return cls(name, name_reference)

  def getName(self,user_id):
    return "Name of %s"%user_id

  def __format(self, msg):
    for id,name in self.names.items():
      msg = msg.replace("[{}]".format(id),name)
    return msg

  def remove(self, user_id):
    del self.names[user_id]
    print("REMOVE: {}".format(user_id))

  def refill(self, users):
    for id in users:
      self.names[id] = users[id]
    print("REFILL: {}".format(users))

  def add(self, users):
    for id in users:
      self.names[id] = users[id]
    print("ADD: {}".format(users))

  def cast(self, msg):
    print("CAST {}: {}".format(self.id, self.format(msg)), flush=True)
    return "-1"

  def cast_resp(self, resp, **locs):
    return self.cast(get_resp(resp, **locs))

  def ack(self, message_id):
    print("ACK: {}".format(message_id))
  
  def getAcks(self, message_id):
    return []

class MDM:
  # dms are based on another (main chat)
  def __init__(self, chat:MChat=None):
    if chat == None:
      self.format = lambda x: x
    else:
      self.format = chat.format

  # TODO: also take an iterable as msg, then send those in chunks based on max msg size
  def send(self,msg,user_id):
    print("SEND {}: {}".format(user_id, self.format(msg)), flush=True)



