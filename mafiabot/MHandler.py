from typing import Union


class MEvent:
  
  def __init__(self, type, data):
    self.type = type
    self.data = data

class MHandler:
  def handle(self, event: MEvent)
    event_type = event.type
    handler_name = "handle_" + event_type
    if not hasattr(self, handler_name):
      #log
      return
    handler = getattr(self, handler_name)
    handler(event)

class MStateHandler(MHandler):

  def __init__(self, mstate):
    self.mstate = mstate

  def handle_vote(self, event):
    # Ensure state is valid for vote
    voter = event.data['voter']
    votee = event.data['votee']
    voter.vote = votee

    if count_votes(self.mstate.players, votee)
      self.mstate.elect(votee)

  def count_votes(players, target: Union[MTarget, None]) -> bool:
    if target == None: # No target
      return False

    nvotes = len([1 for p in players if p.vote == target])
    nplayers = len(self.players)

    if target.target == None:
      return nvotes >= (nplayers+1) // 2 
    else:
      return nvotes > (nplayers) // 2


    