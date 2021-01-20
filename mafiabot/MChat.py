
class CastError(Exception):
  pass

class MChat:

  def __init__(self, group_id):
    self.id = group_id
    self.names = {}

  def destroy(self):
    print("DEL {}".format(self.id))

  @staticmethod
  def new(name):
    return MChat

  def format(self, msg):
    for id,name in self.names.items():
      msg = msg.replace("[{}]".format(id),name)
    return msg

  def getName(self, user_id):
    return "Name of {}".format(user_id)

  def remove(self, user_id):
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

  def ack(self, message_id):
    print("ACK: {}".format(message_id))
  
  def getAcks(self, message_id):
    return []

class MDM:
# No, MDM isn't singleton, it is based on a game?
# We need to somehow generally link chats to get names well.
  def __init__(self):
    if not "instance" in self.__class__.__dict__:
      self.__class__.instance = self
    else:
      raise Exception("MDM is singleton")

  def send(self,msg,user_id):
    print("SEND {}: {}".format(user_id,msg))

  @staticmethod
  def get_dms():
    if not "instance" in MDM.__dict__:
      MDM.instance = MDM()
    return MDM.instance


