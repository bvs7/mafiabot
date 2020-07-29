from .MInfo import *
from .MState import MState
from .MEvent import MPhase

base_id = 1

# TODO: generalize cast/send!

# TODO: timer
# TODO: help
# TODO: Names of players!
# TODO: Start?

def getTarget(text):
  words = text.split()
  target_letter = words[1]
  if not len(target_letter) == 1:
    pass # TODO: catch?
  return target_letter

# Contains MState, checks inputs, fulfills non-event actions
class MGame:

  def __init__(self, rules):
    self.id = base_id
    base_id += 1
    self.rules = rules
    self.started = False
    self.ended = False

  def active(self):
    return self.started and not self.ended

  def main_id(self):
    self.mainChat.id

  def mafia_id(self):
    self.mafiaChat.id

  def handle_main(self, sender_id, command, text, data):
    if command == VOTE_CMD:
      words = text.split()
      voter = sender_id
      votee = data[votee]
      if len(words) >= 3:
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
      target_letter = getTarget(text)
      self.handle_mtarget(sender_id, target_letter)
    elif command == STATUS_CMD:
      self.handle_mafia_status()
    elif command == HELP_CMD:
      self.handle_mafia_help(text)

  def handle_dm(self, sender_id, command, text, data):
    if command == TARGET_CMD:
      target_letter = getTarget(text)
      self.handle_target(sender_id, target_letter)
    elif command == REVEAL_CMD:
      self.handle_reveal(sender_id)
    elif command == STATUS_CMD:
      self.handle_dm_status(sender_id)
    elif command == HELP_CMD:
      self.handle_dm_help(sender_id, text)

  def handle_start(self, mainChat, mafiaChat, dms, users):
    # TODO: rolegen
    ids = users.keys()
    default_roles = ["TOWN","TOWN","MAFIA","TOWN","TOWN"]
    roles = default_roles[:len(ids)]
    mafia_users = {}
    for id, role in zip(ids,roles):
      if role in MAFIA_ROLES:
        mafia_users[id] = users[id]

    self.mainChat = mainChat
    self.mafiaChat = mafiaChat

    self.mainChat.refill(users)
    self.mafiaChat.refill(mafia_users)

    self.dms = dms

    def main_cast(msg:str):
      for id,name in self.mainChat.names.items():
        msg = msg.replace("[{}]".format(id),name)
      self.mainChat.cast(msg)

    def mafia_cast(msg:str):
      for id,name in self.mainChat.names.items():
        msg = msg.replace("[{}]".format(id),name)
      self.mafiaChat.cast(msg)

    def send_dm(msg:str,user_id):
      for id,name in self.mainChat.names.items():
        msg = msg.replace("[{}]".format(id),name)
      self.send_dm(msg, user_id)

    self.state = MState(main_cast, mafia_cast, send_dm, rules)
    self.started = True
    self.state.start(ids,roles)

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
      self.dms.send(default_resp_lib["INVALID_TARGET_PLAYER"],player_id)
      return

    if not self.state.phase == MPhase.NIGHT:
      self.dms.send(default_resp_lib["INVALID_TARGET_PHASE"],player_id)
      return

    try:
      target_number = ord(target_letter.upper())-ord('A')
      if target_number == len(self.state.player_order):
        target_id = "NOTARGET"
      else:
        target_id = self.state.player_order[target_number]
    except KeyError:
      self.dms.send(default_resp_lib["INVALID_TARGET"].format(target_letter=target_letter),player_id)
      return
    if self.state.players.role == "MILKY" and self.state.rules["no_milk_self"] == "ON":
      self.dms.send(default_resp_lib["MILK_SELF"],player_id)
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
      self.dms.send(default_resp_lib["INVALID_REVEAL_PLAYER"],player_id)
      return

    if not self.state.phase == MPhase.DAY:
      self.dms.send(default_resp_lib["INVALID_REVEAL_PHASE"],player_id)
      return

    self.state.reveal(player_id)

  def handle_timer(self, player_id):
    self.main.cast("Timer not implemented yet")
  
  def handle_untimer(self, player_id):
    self.main.cast("Timer not implemented yet")

  def handle_main_help(self, text):
    self.main.cast("Help not implemented yet")

  def handle_mafia_help(self, text):
    self.mafia.cast("Help not implemented yet")
    
  def handle_dm_help(self, player_id, text):
    self.dms.send("Help not implemented yet",player_id)

  def handle_main_status(self):
    msg = self.state.main_status()
    self.main.cast(msg)

  def handle_mafia_status(self):
    msg = self.state.mafia_status()
    self.mafia.cast(msg)

  def handle_dm_status(self, player_id):
    msg = self.state.dm_status(player_id)
    self.dms.send(msg,player_id)


