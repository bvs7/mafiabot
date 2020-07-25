#!/usr/bin/python3
from http.server import BaseHTTPRequestHandler as BaseHandler,HTTPServer

import threading
import time
import json

from MInfo import *

class GroupMeServer:

  def __init__(self):
    

  def do_POST(self,post):
    """Process a POST request from bots watching the chats"""
    post_record = ""
    if 'group_id' in post:
      if post['group_id'] == LOBBY_GROUP_ID:
          post_record += "LOBBY: "
    if 'name' in post:
      post_record += str(post['name']) + "|"
    if 'text' in post:
      post_record += post['text']
    if not post_record == "":
      print(post_record)

    # Check that this is a command
    if post['text'][0:len(ACCESS_KW)] == ACCESS_KW:

      # Check if this was posted by the DM bot
      if '+' in post['group_id']:
        return self.do_DM(post)

      try:
        words = post['text'][len(ACCESS_KW):].split()
        player_id = post['user_id']
        message_id = post['id']
        group_id = post['group_id']
      except KeyError as e:
        log("KeyError:" + str(e))
        return

      if group_id == self.ctrl.lobbyComm.group.id:
        if words[0] in self.ctrl.LOBBY_OPS:
          return self.ctrl.LOBBY_OPS[words[0]](player_id,words,message_id)
      for mstate in self.ctrl.mstates:
          if group_id == mstate.mainComm.group.id:
            if words[0] in self.ctrl.MAIN_OPS:

              # CHECK FOR A VOTE
              if words[0] == VOTE_KW:
                if len(words) < 2: # shouldn't happen?
                  votee = None
                else:
                  if words[1].lower() == "me":
                    votee = player_id
                  elif words[1].lower() == "none":
                    votee = None
                  elif words[1].lower() == "nokill":
                    votee = "0"
                  elif 'attachments' in post:
                    mentions = [a for a in post['attachments'] if a['type'] == 'mentions']
                    if len(mentions) > 0 and 'user_ids' in mentions[0] and len(mentions[0]['user_ids']) >= 1:
                      votee = mentions[0]['user_ids'][0]
                try:
                  player_id = (player_id, votee)
                except:
                  return False

              return self.ctrl.MAIN_OPS[words[0]](mstate,player_id,words,message_id)
          elif group_id == mstate.mafiaComm.group.id:
              if words[0] in self.ctrl.MAFIA_OPS:
                return self.ctrl.MAFIA_OPS[words[0]](mstate,player_id,words,message_id)

    def do_DM(self,DM):
      """Process a DM from a player"""
      log("MServer do_DM",3)
      assert 'sender_id' in DM, "No sender_id in DM for do_DM"
      assert 'text' in DM, "No text in DM for do_DM"
      # Check that this is a valid command
      if (not DM['sender_id'] in MODERATORS) and DM['text'][0:len(ACCESS_KW)] == ACCESS_KW:
        words = DM['text'][len(ACCESS_KW):].split()

        sender_id = DM['sender_id']

        if len(words) > 0:
          try:
            result = self.ctrl.DM_OPS[words[0]](sender_id,words)
          except KeyError as e:
            log("Invalid DM keyword: {}".format(words[0]))
            return False
          return result

if __name__ == "__main__":
  mserver = MServer(GroupComm)

class MainHandler(BaseHandler):

  def do_POST(self):
    try:
      length = int(self.headers['Content-Length'])
      content = self.rfile.read(length).decode('utf-8')
      post = json.loads(content)
    except Exception as e:
      post = {}
      log("failed to load content")

    mserver.do_POST(post)
    return

if __name__ == "__main__":

  server = HTTPServer((ADDRESS,int(PORT)), MainHandler)

  serverThread = threading.Thread(name="Server Thread", target=server.serve_forever)

  serverThread.start()

  while True:
    pass
