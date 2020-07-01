
# This should be defined here?
from .MEx import MPlayerID

class MUser:
  def __init__(self, u_id):
    self.u_id = u_id
    self.focus = None # this is an mstate