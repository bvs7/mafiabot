import threading
import time

from . import MTimer

from ..mafiastate import MRules
from ..chatinterface import MCmd

MIN_PLAYERS = 3
TIMER_MINUTES = 10

# Eh for now have controller and lobby be the same

class MLobby:

  def __init__(self, ctrl, lobby_chat):
    self.ctrl = ctrl
    self.chat = lobby_chat
    self.lobby_cast = self.chat.cast

    self.in_list = {}
    self.start_timer = None
    self.start_msg_id = None
    self.rules = MRules()


  def handle(self, group_id, sender_id, cmd, **kwargs):
    if group_id == self.chat.id:
      return False

    text = kwargs["text"]
    if cmd == MCmd.IN:
      # Get param /in [min]
      words = text.split()
      if len(words) > 1:
        try:
          min_players = int(words[1])
        except ValueError:
          min_players = MIN_PLAYERS
      else:
        min_players = MIN_PLAYERS
      
      self.in_list[sender_id] = min_players

      msg = "[{}] ready to join a game of at least {} players. ".format(sender_id, min_players)
      msg += "{} ready to play.".format(len(self.in_list))
      self.lobby_cast(msg)
      return True

    if cmd == MCmd.OUT:
      for in_p in self.in_list:
        if sender_id == in_p:
          break
      else:
        msg = "You weren't /in"
        self.lobby_cast(msg)
        return True
      del self.in_list[sender_id]
      msg = "Removed [{}]".format(sender_id) # TODO: generalize text
      self.lobby_cast(msg)
      return True

    if cmd == MCmd.START:
      words = text.split()
      min_players = MIN_PLAYERS
      timer_minutes = TIMER_MINUTES
      if len(words) >= 3:
        try:
          min_players = int(words[2])
        except ValueError:
          self.lobby_cast("Couldn't understand min player parameter: {}".format(words[2]))
      if len(words) >= 2:
        try:
          timer_minutes = int(words[1])
        except ValueError:
          self.lobby_cast("Couldn't understand minutes parameter: {}".format(words[1]))

      # Send start msg and record message id
      msg = "Game will start in {} minute{} if at least {} players are in. Like this message to join!".format(
        timer_minutes, 's' if timer_minutes!=1 else '', min_players)
      self.start_msg_id = self.lobby_cast(msg)
      self.start_min_players = min_players
      self.start_timer = MTimer(timer_minutes*60, {0:[self.try_start_game]})
      self.lobby_cast(msg)

    if cmd == MCmd.WATCH:
      if len(self.ctrl.games) == 1:
        name = self.chat.getName(sender_id)
        self.ctrl.games[0].main_chat.add({sender_id:name})
      elif len(self.ctrl.games) == 0:
        self.lobby_cast("Failed to watch, no games")
      else:
        self.lobby_cast("Failed to watch, can't tell which game...")
    
    return False


  def try_start_game(self):

    ack_in_list = [(p_id, self.start_min_players) for p_id in self.chat.getAcks(self.start_msg_id)]

    for (p_id, min_p) in ack_in_list:
      if not p_id in self.in_list:
        self.in_list[p_id] = min_p

    # Sort in_list from largest to smallest min_players
    in_list = self.in_list.items()
    in_list = sorted(in_list, key=lambda x: x[1])
    in_list.reverse()
    # While not done, test for a game, if not, remove largest min_players
    users = {}
    while len(in_list) > 0:
      if len(in_list) >= in_list[0][1]:
        for user_id,_ in in_list:
          users[user_id] = self.chat.getName(user_id)
        break
      else:
        in_list = in_list[1:]

    if len(users) > MIN_PLAYERS:
      self.lobby_cast("Starting game")
      self.ctrl.start_game(self, users, self.rules)
      self.in_list = {}
    else:
      self.lobby_cast("Could not start a game")

  def destroy_game(self, game, id):
    self.ctrl.destroy_game(game, id)
