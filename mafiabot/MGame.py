import time
import json

from .MInfo import *
from .MState import MState, InvalidActionException, EndGameException
from .MEvent import MPhase
from .MRules import MRules
from .MSave import mload, msave
from .MChat import MChat, MDM
from .MPlayer import MPlayerID

# Starts game, creates chats?
# Sticks around after the game to manage chats?
# route these commands into here:
# vote, target, reveal, timer, untimer
# These ones use context? But are general commands usually
# (status), (help)

# Initialized upon "start" in lobby resolving?
# Can create from saved MState, or made new?
# from save:
#  find chats?
#  recreate MState
#  get lobby still?

# TODO: Generalize group_id
MChatType = MChat
MDMType = MDM

# Contains MState, checks inputs, fulfills non-event actions
class MGame:

  def __init__(self, lobby, mstate:MState, 
               main_chat_id, mafia_chat_id):
    self.lobby = lobby
    self.mstate = mstate
    self.main_chat = MChatType(main_chat_id)
    self.mafia_chat = MChatType(mafia_chat_id)
    self.dms = MDM.get_dms() # Singleton!

    self.hook_up()
  
  def hook_up(self):
    self.mstate.cast_main = self.main_chat.cast
    self.mstate.cast_mafia= self.mafia_chat.cast
    self.mstate.send_dm   = self.dms.send
    self.mstate.halt_timer = self.halt_timer

  def halt_timer(self):
    print("Halt timer for MGame {}".format(self))


  @staticmethod
  def new(lobby, rules:MRules=MRules()): 

    state_id = getStateID()
    mstate = MState(state_id, rules)
    main_chat_id = MChatType.new("MAIN CHAT "+str(state_id))
    mafia_chat_id = MChatType.new("MAFIA CHAT "+str(state_id))

    g = MGame(lobby, mstate, main_chat_id, mafia_chat_id)
    return g

  def start(self, users, roleGen):
    """ Inputs, roleGen is a roleGen fn, users is... user ids and nicknames?
    Should we import user nicknames? Yes, eventually have MUser! for generality
    """
    ids = list(users.keys())
    (assignments, contracts) = roleGen(ids)
    ids = [p_id for p_id,r in assignments]
    roles = [MRole(r) for p_id,r in assignments]
    assignments = list(zip(ids,roles))
    # Should roleGen do a better job of returning things?
    mafia_users = {}
    for p_id, role in assignments:
      if role.is_mafia():
        mafia_users[p_id] = users[p_id]

    self.main_chat.add(users)
    self.mafia_chat.add(mafia_users)

    self.mstate.start(assignments, contracts)

  def active(self):
    return self.mstate != None and self.mstate.phase.active()

  def end_game(self,msg):
    # TODO: how to handle this?
    print("END_GAME: ", msg)

  # A message sent to main chat, decide signature?
  def handle_main(self, sender_id:MPlayerID, cmd:MCmd, *args):
    """ This function should be implemented by subclass!
    """
    raise NotImplementedError("Default MGame")
    """
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
    elif command == RULE_CMD:
      self.handle_rule("MAIN", text)

    self.save()
    """

  def handle_mafia(self, sender_id:MPlayerID, cmd:MCmd, *args):
    raise NotImplementedError("Default MGame")
  """
    if command == TARGET_CMD:
      self.handle_mtarget(sender_id, text)
    elif command == STATUS_CMD:
      self.handle_mafia_status()
    elif command == HELP_CMD:
      self.handle_mafia_help(text)
    elif command == RULE_CMD:
      self.handle_rule("MAFIA",text)

    self.save()
    """

  def handle_dm(self, sender_id, cmd:MCmd, *args):
    raise NotImplementedError("Default MGame")
  """
    if command == TARGET_CMD:
      self.handle_target(sender_id, text)
    elif command == REVEAL_CMD:
      self.handle_reveal(sender_id)
    elif command == STATUS_CMD:
      self.handle_dm_status(sender_id)
    elif command == HELP_CMD:
      self.handle_dm_help(sender_id, text)
    elif command == RULE_CMD:
      self.handle_rule(sender_id,text)

    self.save()
    """

  def handle_vote(self,voter_id,votee_id):
    try:
      self.mstate.vote(voter_id, votee_id)
      return True
    except InvalidActionException as iae:
      self.main_chat.cast(iae.msg)
      return False
    except EndGameException as ege:
      self.end_game(ege.msg)
  
  @staticmethod
  def getTarget(text):
    words = text.split()
    target_letter = words[1]
    if not len(target_letter) == 1:
      raise TypeError()
    return target_letter

  def handle_target(self,actor_id,target_id):
    # Get target handled by subclass
    try:
      # If phase is dusk, assume this is an itarget?
      if self.mstate.phase == MPhase.DUSK:
        self.mstate.itarget(actor_id, target_id)
        return True
      else:
        self.mstate.target(actor_id,target_id)
        return True
    except InvalidActionException as iae:
      self.dms.send(iae.msg, actor_id)
      return False
    except EndGameException as ege:
      self.end_game(ege.msg)
      return False

    """
    try:
      target_letter = self.getTarget(text)
      target_number = ord(target_letter.upper())-ord('A')
      if self.mstate.phase == MPhase.DUSK:
        player_order = self.mstate.vengeance.venges
      else:
        player_order = self.mstate.player_order
      if target_number == len(player_order):
        if self.mstate.phase == MPhase.DUSK:
          raise Exception("invalid target {}".format(target_letter))
        target_id = "NOTARGET"
      else:
        target_id = player_order[target_number]
    except Exception as e:
      self.send_dm(resp_lib["INVALID_TARGET"].format(text=text)+"{}".format(e),player_id)
      return
    if (self.mstate.players[player_id].role == "MILKY" and 
        self.mstate.rules["no_milk_self"] == "ON" and
        target_id == player_id):
      self.send_dm(resp_lib["MILK_SELF"],player_id)
      return
    self.mstate.target(player_id, target_id)
    """

  def handle_mtarget(self, targeter_id, target_id):
    try:
      self.mstate.mtarget(targeter_id, target_id)
      return True
    except InvalidActionException as iae:
      self.mafia_chat.cast(iae.msg)
      return False
    except EndGameException as ege:
      self.end_game(ege.msg)
 
    """
    try:
      target_letter = self.getTarget(text)
      target_number = ord(target_letter.upper())-ord('A')
      if target_number == len(self.mstate.player_order):
        target_id = "NOTARGET"
      else:
        target_id = self.mstate.player_order[target_number]
    except Exception:
      self.mafia_cast(resp_lib["INVALID_MTARGET"].format(text=text))
      return
    
    role = self.mstate.players[player_id].role
    if role == "GOON" and target_id != "NOTARGET":
      self.mafia_cast(resp_lib["INVALID_MTARGET_GOON"])
      return

    self.mstate.mtarget(player_id, target_id)
    """

  def handle_reveal(self, reveal_id):

    try:
      self.mstate.reveal(reveal_id)
      return True
    except InvalidActionException as iae:
      self.dms.send(iae.msg, reveal_id)
      return False
    except EndGameException as ege:
      self.end_game(ege.msg)
      return False

    # if not (player_id in self.mstate.players and self.mstate.players[player_id].role == "CELEB"):
    #   self.send_dm(resp_lib["INVALID_REVEAL_PLAYER"],player_id)
    #   return

    # if not self.mstate.phase == MPhase.DAY:
    #   self.send_dm(resp_lib["INVALID_REVEAL_PHASE"],player_id)
    #   return

    # self.mstate.reveal(player_id)

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
    msg = self.mstate.main_status()
    self.main_chat.cast(msg)

  def handle_mafia_status(self):
    msg = self.mstate.mafia_status()
    self.mafia_chat.cast(msg)

  def handle_dm_status(self, player_id):
    msg = self.mstate.dm_status(player_id)
    self.dms.send(msg,player_id)

  def handle_rule(self, sender, text):
    """ Return the rule for a specific rule or list of rules """
    pass
    # msg = ""
    # words = text.split()
    # if len(words) == 1:
    #   msg = self.mstate.rules.describe(has_expl=False)
    # elif words[1] in MRules.RULE_BOOK:
    #   rule = words[1]
    #   msg = "{}:\n".format(rule)
    #   msg += self.mstate.rules[rule].explRule()
    # elif words[1] == "long":
    #   msg = self.mstate.rules.describe(has_expl=True)
    
    # if sender == "MAIN":
    #   self.main_cast(msg)
    # elif sender == "MAFIA":
    #   self.mafia_cast(msg)
    # else:
    #   self.send_dm(msg, sender)

  def save(self):
    f = open("../data/game_{}.maf".format(self.mstate.id),"w")
    msave(self,f)
    f.close()

  def to_json(self):
    d = {
      "main_chat_id": self.main_chat.id,
      "mafia_chat_id": self.mafia_chat.id,
      "mstate":self.mstate,
    }
    return d

  @staticmethod
  def load(lobby, f):
    mgame = mload(f)
    mgame.lobby = lobby
    return mgame

  @staticmethod
  def from_json(d):
    lobby = None
    mgame = MGame(lobby, d["mstate"], 
                  d["main_chat_id"], d["mafia_chat"])
    return mgame