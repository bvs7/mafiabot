import time

from .MInfo import *
from .MState import MState
from .MEvent import MPhase
from .MRoleGen import roleGen

# TODO: Can MGame be folded into MState?
# TODO: Unique ID
# TODO: generalize cast/send!

# TODO: timer
# TODO: help

# TODO: Don't destroy group on end

# Contains MState, checks inputs, fulfills non-event actions
class MGame:

  def __init__(self, MChatType, dms, rules, end_callback, users):
    self.main_chat = MChatType.new("MAIN CHAT")
    self.mafia_chat = MChatType.new("MAFIA CHAT")
    self.dms = dms
    self.rules = rules

    def main_cast(msg:str):
      self.main_chat.cast(self.main_chat.format(msg))
    def mafia_cast(msg:str):
      self.mafia_chat.cast(self.main_chat.format(msg))
    def send_dm(msg:str,user_id):
      self.dms.send(self.main_chat.format(msg), user_id)
    def end_callback_(e):
      end_callback(self, e)

    ids = list(users.keys())
    (ids, roles, contracts) = roleGen(ids)
    mafia_users = {}
    for id, role in zip(ids,roles):
      if role in MAFIA_ROLES:
        mafia_users[id] = users[id]

    self.main_chat.refill(users)
    self.mafia_chat.refill(mafia_users)
    self.state = MState(main_cast, mafia_cast, send_dm, self.rules, end_callback_, ids, roles, contracts)

  def active(self):
    return self.state.active

  def main_id(self):
    return self.main_chat.id

  def mafia_id(self):
    return self.mafia_chat.id

  def handle_main(self, sender_id, command, text, data):
    if command == VOTE_CMD:
      words = text.split()
      voter = sender_id
      votee = None
      if len(words) >= 2:
        # TODO: Generalize language
        if words[1].lower() == "me":
          votee = sender_id
        elif words[1].lower() == "none":
          votee = None
        elif words[1].lower() == "nokill":
          votee = "NOTARGET"
        elif 'attachments' in data:
          mentions = [a for a in data['attachments'] if a['type'] == 'mentions']
          if len(mentions) > 0 and 'user_ids' in mentions[0] and len(mentions[0]['user_ids']) >= 1:
            votee = mentions[0]['user_ids'][0]
      self.handle_vote(voter,votee)
    elif command == STATUS_CMD:
      self.handle_main_status()
    elif command == HELP_CMD:
      self.handle_main_help(text)
    elif command == TIMER_CMD:
      self.handle_timer(sender_id)
    elif command == UNTIMER_CMD:
      self.handle_untimer(sender_id)

  def handle_mafia(self, sender_id, command, text, data):
    if command == TARGET_CMD:
      self.handle_mtarget(sender_id, text)
    elif command == STATUS_CMD:
      self.handle_mafia_status()
    elif command == HELP_CMD:
      self.handle_mafia_help(text)

  def handle_dm(self, sender_id, command, text, data):
    if command == TARGET_CMD:
      self.handle_target(sender_id, text)
    elif command == REVEAL_CMD:
      self.handle_reveal(sender_id)
    elif command == STATUS_CMD:
      self.handle_dm_status(sender_id)
    elif command == HELP_CMD:
      self.handle_dm_help(sender_id, text)

  def handle_vote(self,player_id,target_id):
    if not player_id in self.state.players:
      self.main_chat.cast(default_resp_lib["INVALID_VOTE_PLAYER"].format(player_id=player_id))
      return

    if not self.state.phase == MPhase.DAY:
      self.main_chat.cast(default_resp_lib["INVALID_VOTE_PHASE"])
      return

    self.state.vote(player_id, target_id)
  
  @staticmethod
  def getTarget(text):
    words = text.split()
    target_letter = words[1]
    if not len(target_letter) == 1:
      raise TypeError()
    return target_letter

  def handle_target(self,player_id, text):
    if self.state.phase == MPhase.NIGHT:    
      if not (player_id in self.state.players and self.state.players[player_id].role in TARGETING_ROLES):
        self.dms.send(default_resp_lib["INVALID_TARGET_PLAYER"],player_id)
        return
    elif self.state.phase == MPhase.DUSK:
      if not (player_id in self.state.players and self.state.players[player_id].role == "IDIOT"):
        self.dms.send(default_resp_lib["INVALID_TARGET_PLAYER"],player_id)
        return
    else:
      self.dms.send(default_resp_lib["INVALID_TARGET_PHASE"],player_id)
      return

    try:
      target_letter = self.getTarget(text)
      target_number = ord(target_letter.upper())-ord('A')
      if target_number == len(self.state.player_order):
        target_id = "NOTARGET"
      else:
        target_id = self.state.player_order[target_number]
    except Exception:
      self.dms.send(default_resp_lib["INVALID_TARGET"].format(text=text),player_id)
      return
    if self.state.players[player_id].role == "MILKY" and self.state.rules["no_milk_self"] == "ON":
      self.dms.send(default_resp_lib["MILK_SELF"],player_id)
      return
    self.state.target(player_id, target_id)

  def handle_mtarget(self, player_id, text):
    if not (player_id in self.state.players and self.state.players[player_id].role in MAFIA_ROLES):
      self.mafia_chat.cast(default_resp_lib["INVALID_MTARGET_PLAYER"])
      return

    if not self.state.phase == MPhase.NIGHT:
      self.mafia_chat.cast(default_resp_lib["INVALID_MTARGET_PHASE"])
      return
    
    try:
      target_letter = self.getTarget(text)
      target_number = ord(target_letter.upper())-ord('A')
      if target_number == len(self.state.player_order):
        target_id = "NOTARGET"
      else:
        target_id = self.state.player_order[target_number]
    except Exception:
      self.mafia_chat.cast(default_resp_lib["INVALID_MTARGET"].format(text=text))
      return
    
    role = self.state.players[player_id].role
    if role == "GOON" and target_id != "NOTARGET":
      self.mafia_chat.cast(default_resp_lib["INVALID_MTARGET_GOON"])
      return

    self.state.mtarget(player_id, target_id)

  def handle_reveal(self, player_id):
    if not (player_id in self.state.players and self.state.players[player_id].role):
      self.dms.send(default_resp_lib["INVALID_REVEAL_PLAYER"],player_id)
      return

    if not self.state.phase == MPhase.DAY:
      self.dms.send(default_resp_lib["INVALID_REVEAL_PHASE"],player_id)
      return

    self.state.reveal(player_id)

  def handle_timer(self, player_id):
    self.main_chat.cast("Timer not implemented yet")
  
  def handle_untimer(self, player_id):
    self.main_chat.cast("Timer not implemented yet")

  def handle_main_help(self, text):
    self.main_chat.cast("Help not implemented yet")

  def handle_mafia_help(self, text):
    self.mafia_chat.cast("Help not implemented yet")
    
  def handle_dm_help(self, player_id, text):
    self.dms.send("Help not implemented yet",player_id)

  def handle_main_status(self):
    msg = self.state.main_status()
    self.main_chat.cast(msg)

  def handle_mafia_status(self):
    msg = self.state.mafia_status()
    self.mafia_chat.cast(msg)

  def handle_dm_status(self, player_id):
    msg = self.state.dm_status(player_id)
    self.dms.send(msg,player_id)

  def end(self):
    self.main_chat.destroy()
    self.mafia_chat.destroy()


