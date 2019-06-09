from .MPlayer import MPlayer
from .MHandler import MEvent

class MState:
  """State and handlers for Mafia game"""

  def __init__(self):
    self.day = 0
    self.phase = "Init"
    self.state = "Init"

    self.handlers = []
    self.eventQueue = []

  def queueEvent(self, event):
    self.eventQueue.append(event)

  def vote(self, voter: MPlayer, votee: MPlayer):
    if voter.vote is votee:
      # Already voted
      return
    else:
      voteEvent = MEvent('vote', {'voter':voter, 'votee':votee})
      self.queueEvent(voteEvent)




