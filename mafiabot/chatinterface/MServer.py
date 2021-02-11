#!/usr/bin/python3
from http.server import BaseHTTPRequestHandler as BaseHandler,HTTPServer

import threading
import time
import json
from collections import deque

from ..util import VEnum

class MCmd(VEnum):
  VOTE = "vote"
  TARGET = "target"
  REVEAL = "reveal"
  TIMER = "timer"
  UNTIMER = "untimer"
  HELP = "help"
  STATUS = "status"
  START = "start"
  IN = "in"
  OUT = "out"
  RULE = "rule"
  WATCH = "watch"
  FOCUS = "focus"
  END = "end"

  @staticmethod
  def parseCmd(s):
    m = MCmd.__members__
    for k,v in m:
      if v == s:
        return k
    return None

  @classmethod
  def main(cls):
    return {
      MCmd.VOTE,
      MCmd.TIMER,
      MCmd.UNTIMER,
      MCmd.STATUS,
      MCmd.HELP,
      MCmd.END
    }

  def is_main(self):
    return self in self.main()

  @classmethod
  def mafia(cls):
    return {
      MCmd.TARGET,
      MCmd.STATUS,
      MCmd.HELP,
    }

  def is_mafia(self):
    return self in self.mafia()

  @classmethod
  def dm(cls):
    return {
      MCmd.TARGET,
      MCmd.REVEAL,
      MCmd.STATUS,
      MCmd.HELP
    }
  
  def is_game_dm(self):
    return self in self.dm()

  @classmethod
  def lobby(cls):
    return {
      MCmd.START,
      MCmd.IN,
      MCmd.OUT,
      MCmd.WATCH,
      MCmd.STATUS,
      MCmd.HELP,
    }

  def is_lobby(self):
    return self in self.lobby()
      

ACCESS_KW = "/"

class MServer:

  def __init__(self, handle_chat, handle_dm):
    pass

  def chat(self):
    pass

  def dm(self):
    pass

  def run(self):
    raise NotImplementedError("Default MServer")

class TestMServer(MServer):

  def __init__(self, handle_chat, handle_dm, lines:deque=None):
    self.handle_chat = handle_chat
    self.handle_dm = handle_dm

    self.active = True
    self.lines = lines

  @staticmethod
  def from_file(handle_chat, handle_dm, input_fname):
    with open(input_fname, 'r') as f:
      lines = deque(f.readlines())
    return TestMServer(handle_chat, handle_dm, lines)

  def append(self, line):
    if not self.lines:
      self.lines = deque()
    self.lines.append(line)
      
  def chat(self, line=None):
    if not line:
      text = input("Text: ")
    else:
      words = deque(line.split())
      text = words.popleft()
    if text[0:len(ACCESS_KW)] == ACCESS_KW:
      if not line:
        group_id = input("Group_id: ")
        sender_id = input("Sender_id: ")
      else:
        group_id = words.popleft()
        sender_id = words.popleft()
      command = text.split()[0][len(ACCESS_KW):]
      if not line:
        j = input("Data (json): ")
      else:
        if len(words) == 0:
          j = ""
        else:
          j = words.popleft()
      if j in "":
        data = {}
      elif j[0:len('vote')] == 'vote':
        votee = j.split()[1]
        data = {'attachments':[{'type':'mentions','user_ids':[votee]}]}
      else:
        data = json.loads(j)
      self.handle_chat(group_id, sender_id, command, text=text, data=data)

  def dm(self, line=None):
    if not line:
      text = input("Text: ")
    else:
      words = deque(line.split())
      text = words.popleft()
    if text[0:len(ACCESS_KW)] == ACCESS_KW:
      if not line:
        sender_id = input("Sender_id: ")
      else:
        sender_id = words.popleft()
      command = text.split()[0][len(ACCESS_KW):]
      if not line:
        j = input("Data (json): ")
      else:
        if len(words) == 0:
          j = ""
        else:
          j = words.popleft()
      if j == "":
        data = {}
      else:
        data = json.loads(j)
      self.handle_dm(sender_id, command, text=text, data=data)

  def parse_lines(self) -> bool:
    if self.lines:
      line = self.lines.popleft()
      self.parse(line)
      return len(self.lines) != 0
    return False

  def parse(self, line=None):
    if not line:
      chat_dm = input("Chat/DM, [c]/d: ")
      rest_line = None
    else:
      words = line.split()
      chat_dm = words[0]
      rest_line = " ".join(words[1:])
      if chat_dm == ACCESS_KW:
        chat_dm = ""
    if chat_dm == "quit":
      self.active = False
    if chat_dm[0] == "#":
      return
    if chat_dm in ["", "c", "chat"]:
      self.chat(rest_line)
    else:
      self.dm(rest_line)

  def start(self):
    thread = threading.Thread(target=self.run)
    thread.start()

  def run(self):
    if not self.lines:
      while self.active:
        self.parse()
        time.sleep(.5)
    else:
      while self.active:
        self.active = self.parse_lines()