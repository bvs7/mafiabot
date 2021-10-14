# Subscriber to a single game's events. Sends messages to the communication system (GroupMe, etc)
# Responds to events and requests?
# Also subscribe to non-event requests?

import inspect
from typing import Dict

from pubsub import pub

# String lookup method?

def get_msg(name):
  pass

def is_event_response(name):
  return name[0:3] == "on_"

def get_event_name(name):
  return name[3:]


# defines topic message data
class MResp():

  def __init__(self, game_id:str):
    self.game_id = game_id

  def subscribe(self):
    event_base = f"game.{self.game_id}.event."
    # Subscribe to implemented events for this game
    for name in dir(self):
      if is_event_response(name):
        pub.subscribe(getattr(self, name), event_base+get_event_name(name))

  def on_start(self, player_info): pass
  def on_vote(self, voter, votee, f_votee=None): pass
  def on_mtarget(self, actor, target, guilty): pass
  def on_target(self, actor, target): pass
  def on_reveal(self, actor, is_reminder=False): pass
  def on_timer(self): pass
  def on_elect(self, target, hammer, guilty, is_idiot=False): pass
  def on_kill(self, target, actor, guilty): pass
  def on_vengeance(self, target, actor): pass
  def on_eliminate(self, target, actor, guilty): pass
  def on_refocus(self, actor, new_role): pass
  def on_charge(self, actor, charge): pass
  def on_block(self, actor, target, is_useful): pass
  def on_stun(self, actor, targets): pass
  def on_save(self, actor, target, is_blocked, is_effective): pass
  def on_milk(self, actor, target, is_blocked, is_effective): pass
  def on_investigate(self, actor, target, is_blocked): pass
  def on_win(self, winning_team, winner=None, role=None): pass

class PrintMResp(MResp):
  def __init__(self, game_id:str):
    super().__init__(game_id)
    pub.subscribe(self.on_event, pub.ALL_TOPICS)

  def on_event(self, topic=pub.AUTO_TOPIC, *args, **kwargs):
    print(topic.getName(), args, kwargs)