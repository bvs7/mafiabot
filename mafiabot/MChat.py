
class CastError(Exception):
  pass

class MChat:

  def __init__(self, group_id, name_reference=None):
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

  @staticmethod
  def new(name,): # Just return id?
    return MChat(name).id

  def getName(self,user_id):
    return self.names[user_id]

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
    print("CAST {}: {}".format(self.id, self.format(msg)))
    return "-1"

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

  def send(self,msg,user_id):
    print("SEND {}: {}".format(user_id, self.format(msg)))



