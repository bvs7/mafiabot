
class CastError(Exception):
  pass

class MChat:

  def __init__(self, group_id):
    pass

  def new(name):
    chat = MChat(name)
    return chat

  def getName(self, user_id):
    raise NotImplementedError("Default MChat")

  def remove(self, user_id):
    raise NotImplementedError("Default MChat")

  def refill(self, users):
    raise NotImplementedError("Default MChat")

  def add(self, users):
    raise NotImplementedError("Default MChat")

  def cast(self,msg):
    raise NotImplementedError("Default MChat")

  def ack(self, message_id):
    raise NotImplementedError("Default MChat")

  def getAcks(self, message_id):
    raise NotImplementedError("Default MChat")

  def destroy(self):
    pass

class MDM:

  def __init__(self):
    raise NotImplementedError("Default MDM")

  def send(self, msg, user_id):
    raise NotImplementedError("Default MDM")


class TestMChat(MChat):
  
  def __init__(self, group_id):
    self.id = group_id

  def destroy(self):
    print("DEL {}".format(self.id))
    
  def new(name):
    return TestMChat(name)

  def format(self, msg):
    return msg

  def getName(self, user_id):
    return "Name of {}".format(user_id)

  def remove(self, user_id):
    print("REMOVE: {}".format(user_id))

  def refill(self, users):
    print("REFILL: {}".format(users))

  def add(self, users):
    print("ADD: {}".format(users))

  def cast(self, msg):
    print("CAST {}: {}".format(self.id, msg))

  def ack(self, message_id):
    print("ACK: {}".format(message_id))
  
  def getAcks(self, message_id):
    return []

class TestMDM(MDM):
  
  def __init__(self):
    pass

  def send(self,msg,user_id):
    print("SEND {}: {}".format(user_id,msg))