
class CastError(Exception):
  pass

class MChat:

  def cast(self,msg):
    raise NotImplementedError("Default MChat")

class MDM:

  def id(self):
    raise NotImplementedError("Default MDM")

  def send(self, msg):
    raise NotImplementedError("Default MDM")