import time
import json

from typing import NewType

from ..mafiastate import MState, MRules, MRole, MPlayerID, InvalidActionException, EndGameException, MPhase, mload 
from ..chatinterface import MChat, MDM, MCmd

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

class DeleteGameException(Exception):
  pass

# Contains MState, checks inputs, fulfills non-event actions
class MGame:

  def __init__(self, ctrl, game_id, mstate:MState, lobby=None,
               main_chat_id=None, mafia_chat_id=None):
    # Search for saved game?
    print("New MGame", flush = True)
    self.ctrl = ctrl
    self.id = game_id
    self.mstate = mstate
    if main_chat_id == None:
      main_chat_id = self.ctrl.MChatType.new("MAIN CHAT {}".format(self.id))
    self.main_chat = self.ctrl.MChatType(main_chat_id)
    if mafia_chat_id == None:
      mafia_chat_id = self.ctrl.MChatType.new("MAFIA CHAT {}".format(self.id))
    self.mafia_chat = self.ctrl.MChatType(mafia_chat_id, name_reference=self.main_chat)
    self.dms = self.ctrl.MDMType(self.main_chat)

    self.hook_up()
  
  def hook_up(self):
    self.mstate.cast_main = self.main_chat.cast
    self.mstate.cast_mafia= self.mafia_chat.cast
    self.mstate.send_dm   = self.dms.send
    self.mstate.halt_timer = self.halt_timer


  @classmethod
  def new(cls, ctrl, rules:MRules=MRules(), lobby=None): 

    game_id = 1#getNewGameID()
    mstate = MState(rules)

    g = cls(ctrl, game_id, mstate, lobby)
    print("Return: ", g, flush = True)
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
      if MRole(role).is_mafia():
        mafia_users[p_id] = users[p_id]

    self.main_chat.add(users)
    self.mafia_chat.add(mafia_users)

    try:
      self.mstate.start(assignments, contracts)
    except Exception as e:
      print(e, flush=True)

  def active(self):
    return self.mstate != None and self.mstate.phase.active()

  def end_game(self,msg):
    # TODO: how to handle this?
    print("END_GAME: ", msg)

  # decorator for end of game or invalid action
  def handle_chat(self, group_id, sender_id:MPlayerID, cmd:MCmd, **kwargs):
    if group_id == self.main_chat.id and cmd.is_main():
      try:
        self.handle_main(sender_id, cmd, **kwargs)
      except Exception as e:
        raise e # TODO handle better?
      return True
    elif group_id == self.mafia_chat.id and cmd.is_mafia():
      try:
        self.handle_mafia(sender_id, cmd, **kwargs)
      except Exception as e:
        raise e # TODO handle better?
      return True

  # A message sent to main chat, decide signature?
  def handle_main(self, sender_id:MPlayerID, cmd:MCmd, **kwargs):
    raise NotImplementedError("Default MGame")

  def handle_mafia(self, sender_id:MPlayerID, cmd:MCmd, **kwargs):
    raise NotImplementedError("Default MGame")

  def handle_dm(self, sender_id, cmd:MCmd, **kwargs):
    raise NotImplementedError("Default MGame")


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

  def handle_mtarget(self, targeter_id, target_id):
    try:
      self.mstate.mtarget(targeter_id, target_id)
      return True
    except InvalidActionException as iae:
      self.mafia_chat.cast(iae.msg)
      return False
    except EndGameException as ege:
      self.end_game(ege.msg)


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
    
  def halt_timer(self):
    print("Halt timer for MGame {}".format(self))

  def handle_timer(self, player_id):
    self.main_chat.cast("Timer not implemented yet")
  
  def handle_untimer(self, player_id):
    self.main_chat.cast("Timer not implemented yet")

  def handle_main_help(self, sender_id, text):
    self.main_chat.cast("Help not implemented yet")

  def handle_mafia_help(self, sender_id, text):
    self.mafia_chat.cast("Help not implemented yet")
    
  def handle_dm_help(self, sender_id, text):
    self.dms.send("Help not implemented yet",sender_id)

  def handle_main_status(self, sender_id, text):
    msg = self.mstate.main_status()
    self.main_chat.cast(msg)

  def handle_mafia_status(self, sender_id, text):
    msg = self.mstate.mafia_status()
    self.mafia_chat.cast(msg)

  def handle_dm_status(self, sender_id, text):
    msg = self.mstate.dm_status(sender_id)
    self.dms.send(msg,sender_id)

  def handle_rule(self, sender_id, text):
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

  # Do some __del__
  def destroy(self):
    self.mstate.destroy()
    self.main_chat.destroy()
    self.mafia_chat.destroy()
    raise DeleteGameException()

  def handle_end(self, sender_id, *args):
    if self.mstate.phase == MPhase.END:
      self.destroy()

  def save(self):
    # f = open("game_{}.maf".format(self.id),"w")
    # msave(self,f)
    # f.close()
    pass

  def to_json(self):
    d = {
      "main_chat_id": self.main_chat.id,
      "mafia_chat_id": self.mafia_chat.id,
      "mgame_id": self.id,
      "mstate":self.mstate,
    }
    return d

  @classmethod
  def load(cls, f):
    mgame = mload(f)
    return mgame

  @classmethod
  def from_json(cls,d):
    mgame = cls(d["mgame_id"],d["mstate"], d["main_chat_id"], d["mafia_chat_id"])
    return mgame