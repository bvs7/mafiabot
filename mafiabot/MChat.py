
class CastError(Exception):
  pass

class MChat:

  def __init__(self, group_id):
    pass

  def new(name):
    chat = MChat(name)
    return chat

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

class MDM:

  def __init__(self):
    raise NotImplementedError("Default MDM")

  def send(self, msg, user_id):
    raise NotImplementedError("Default MDM")