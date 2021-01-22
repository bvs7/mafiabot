#!/usr/bin/python3
from http.server import BaseHTTPRequestHandler as BaseHandler,HTTPServer

import threading
import time
import json

from .MInfo import *

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

  def __init__(self, handle_chat, handle_dm):
    self.handle_chat = handle_chat
    self.handle_dm = handle_dm

    self.active = True

    thread = threading.Thread(target=self.loop)
    thread.start()


  def chat(self):
    text = input("Text: ")
    if text[0:len(ACCESS_KW)] == ACCESS_KW:
      group_id = input("Group_id: ")
      sender_id = input("Sender_id: ")
      command = text.split()[0][len(ACCESS_KW):]
      j = input("Data (json): ")
      if j == "":
        data = {}
      elif j[0:len('vote')] == 'vote':
        votee = j.split()[1]
        data = {'attachments':[{'type':'mentions','user_ids':[votee]}]}
      else:
        data = json.loads(j)
      self.handle_chat(group_id, sender_id, command, text, data)

  def dm(self):
    text = input("Text: ")
    if text[0:len(ACCESS_KW)] == ACCESS_KW:
      sender_id = input("Sender_id: ")
      command = text.split()[0][len(ACCESS_KW):]
      j = input("Data (json): ")
      if j == "":
        data = {}
      else:
        data = json.loads(j)
      self.handle_dm(sender_id, command, text, data)

  def run(self):
    while self.active:
      chat_dm = input("Chat/DM, [c]/d: ")
      if chat_dm in ["", "c", "chat"]:
        self.chat()
      else:
        self.dm()
      if chat_dm == "quit":
        self.active = False
      time.sleep(.5)