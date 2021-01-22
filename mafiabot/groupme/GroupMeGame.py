from mafiabot import MGame, resp_lib, MRole, MPlayer, MPlayerID, MPhase, MCmd

from .GroupMeChat import GroupMeChat, GroupMeDM


class GroupMeGame(MGame):

  MChatType = GroupMeChat
  MDMType = GroupMeDM

  def handle_main(self, sender_id:MPlayerID, cmd:MCmd, text="", data={}):
    if cmd == MCmd.VOTE:
      self.handle_vote(sender_id, text, data)
    elif cmd == MCmd.STATUS:
      self.handle_main_status(sender_id, text)
    elif cmd == MCmd.HELP:
      self.handle_main_help(sender_id, text)
    elif cmd == MCmd.TIMER:
      self.handle_timer(sender_id)
    elif cmd == MCmd.UNTIMER:
      self.handle_untimer(sender_id)

  def handle_mafia(self, sender_id:MPlayerID, cmd:MCmd, text="", data={}):
    if cmd == MCmd.TARGET:
      self.handle_mtarget(sender_id, text)
    elif cmd == MCmd.STATUS:
      self.handle_mafia_status(sender_id, text)
    elif cmd == MCmd.HELP:
      self.handle_mafia_help(sender_id, text)

  def handle_dm(self, sender_id:MPlayerID, cmd:MCmd, text="", data={}):
    if cmd == MCmd.TARGET:
      self.handle_target(sender_id, text)
    elif cmd == MCmd.REVEAL:
      self.handle_reveal(sender_id)
    elif cmd == MCmd.STATUS:
      self.handle_dm_status(sender_id, text)
    elif cmd == MCmd.HELP:
      self.handle_dm_help(sender_id, text)

  def handle_vote(self, sender_id, text, data):
    words = text.split()
    voter_id = sender_id
    votee_id = None
    if len(words) >= 2:
      # TODO: Generalize language (me, none, nokill)
      if words[1].lower() == "me":
        votee_id = sender_id
      elif words[1].lower() == "none":
        votee_id = None
      elif words[1].lower() == "nokill":
        votee_id = MPlayer.NOTARGET
      elif 'attachments' in data:
        mentions = [a for a in data['attachments'] if a['type'] == 'mentions']
        if len(mentions) > 0 and 'user_ids' in mentions[0] and len(mentions[0]['user_ids']) >= 1:
          votee_id = mentions[0]['user_ids'][0]

    super().handle_vote(voter_id, votee_id)

  @staticmethod
  def getTarget(text):
    words = text.split()
    target_letter = words[1].upper()
    if not target_letter.isalpha():
      raise TypeError()
    target_number = 0
    m = 1
    while len(target_letter) > 0:
      target_number += (ord(target_letter[-1])-ord('A')) * m
      m *= m
      target_letter = target_letter[:-1]

    return target_letter

  def handle_mtarget(self, sender_id, text):
    targeter_id = sender_id
    try:
      target_number = self.getTarget(text)
      if target_number == len(self.mstate.players.keys()):
        target_id = MPlayer.NOTARGET
      else:
        target_id = list(self.mstate.players.keys())[target_number]
    except Exception:
      self.mafia_chat.cast(resp_lib["INVALID_MTARGET"].format(text=text))
      return False
    
    return super().handle_mtarget(targeter_id, target_id)

  def handle_target(self, sender_id, text):
    itarget = self.mstate.phase == MPhase.DUSK
    try:
      target_number = self.getTarget(text)
      if itarget:
        player_order = self.mstate.vengeance.venges
      else:
        player_order = list(self.mstate.players.key())
      if target_number == len(player_order):
        target_id = MPlayer.NOTARGET
      else:
        target_id = player_order[target_number]
    except Exception as e:
      self.dms.send(resp_lib["INVALID_TARGET"].format(text=text)+"{}".format(e),sender_id)
      return False

    return super().handle_target(sender_id, target_id)
