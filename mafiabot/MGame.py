from .MInfo import *
from .MState import MState
from .MEvent import MPhase

# TODO: timer
# TODO: help
# TODO: Names of players!

# Contains MState, checks inputs, fulfills non-event actions
class MGame:

  def __init__(self, main, mafia, dms, rules):

    self.main = main
    self.mafia = mafia
    self.dms = dms
    self.rules = rules

    def send_dm(msg, p_id):
      self.dms[p_id].send(msg)

    self.state = MState(self.main.cast, self.mafia.cast, send_dm, rules)

  def handle_vote(self,player_id,target_id):
    if not player_id in self.state.players:
      self.main.cast(default_resp_lib["INVALID_VOTE_PLAYER"].format(player_id))
      return

    if not self.state.phase == MPhase.DAY:
      self.main.cast(default_resp_lib["INVALID_VOTE_PHASE"])
      return

    self.state.vote(player_id, target_id)
    
  def handle_target(self,player_id,target_letter):
    if not (player_id in self.state.players and self.state.players[player_id].role in TARGETING_ROLES):
      self.dms[player_id].send(default_resp_lib["INVALID_TARGET_PLAYER"])
      return

    if not self.state.phase == MPhase.NIGHT:
      self.dms[player_id].send(default_resp_lib["INVALID_TARGET_PHASE"])
      return

    try:
      target_number = ord(target_letter.upper())-ord('A')
      if target_number == len(self.state.player_order):
        target_id = "NOTARGET"
      else:
        target_id = self.state.player_order[target_number]
    except KeyError:
      self.dms[player_id].send(default_resp_lib["INVALID_TARGET"].format(target_letter=target_letter))
      return
    if self.state.players[player_id].role == "MILKY" and self.state.rules["no_milk_self"] == "ON":
      self.dms[player_id].send(default_resp_lib["MILK_SELF"])
      return
    self.state.target(player_id, target_id)

  def handle_mtarget(self, player_id, target_letter):
    if not (player_id in self.state.players and self.state.players[player_id].role in MAFIA_ROLES):
      self.mafia.cast(default_resp_lib["INVALID_MTARGET_PLAYER"])
      return

    if not self.state.phase == MPhase.NIGHT:
      self.mafia.cast(default_resp_lib["INVALID_MTARGET_PHASE"])
      return
    
    try:
      target_number = ord(target_letter.upper())-ord('A')
      if target_number == len(self.state.player_order):
        target_id = "NOTARGET"
      else:
        target_id = self.state.player_order[target_number]
    except KeyError:
      self.mafia.cast(default_resp_lib["INVALID_MTARGET"].format(target_letter=target_letter))
      return
    
    role = self.state.players[player_id].role
    if role == "GOON" and target_id != "NOTARGET":
      self.mafia.cast(default_resp_lib["INVALID_MTARGET_GOON"])
      return

    self.state.mtarget(player_id, target_id)

  def handle_reveal(self, player_id):
    if not (player_id in self.state.players and self.state.players[player_id].role):
      self.dms[player_id].send(default_resp_lib["INVALID_REVEAL_PLAYER"])
      return

    if not self.state.phase == MPhase.DAY:
      self.dms[player_id].send(default_resp_lib["INVALID_REVEAL_PHASE"])
      return

    self.state.reveal(player_id)

  def handle_timer(self, player_id):
    self.main.cast("Timer not implemented yet")
  
  def handle_untimer(self, player_id):
    self.main.cast("Timer not implemented yet")

  def handle_main_help(self, text):
    self.main.cast("Help not implemented yet")

  def handle_main_status(self):
    msg = self.state.main_status()
    self.main.cast(msg)

  def handle_mafia_status(self):
    msg = self.state.mafia_status()
    self.mafia.cast(msg)

  def handle_dm_status(self, player_id):
    msg = self.state.dm_status(player_id)
    self.dms[player_id].send(msg)


