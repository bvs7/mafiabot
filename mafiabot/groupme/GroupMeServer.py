#!/usr/bin/python3
from flask import Flask, request
import json

from mafiabot import ACCESS_KW, MServer

class GroupMeServer(MServer):

  def __init__(self, handle_chat, handle_dm):
    self.handle_chat = handle_chat
    self.handle_dm = handle_dm
    self.__app = Flask('mafiabot')

    self.__app.add_url_rule('/', 'chat', self.chat, methods=['POST'])
    self.__app.add_url_rule('/dm', 'dm', self.dm, methods=['POST'])

  def chat(self):
    print(request.data)
    data = json.loads(request.data.decode('utf-8'))
    text = data['text']
    if text[0:len(ACCESS_KW)] == ACCESS_KW:
      group_id = data['group_id']
      sender_id = data['sender_id']
      command = text.split()[0][len(ACCESS_KW):]
      self.handle_chat(group_id, sender_id, command, text, data)
    return "ok"

  def dm(self):
    data = json.loads(request.data.decode('utf-8'))
    text = data['text']
    if text[0:len(ACCESS_KW)] == ACCESS_KW:
      sender_id = data['sender_id']
      command = text.split()[0][len(ACCESS_KW):]
      self.handle_dm(sender_id, command, text, data)
    return "ok"

  def run(self):
    self.__app.run(host="0.0.0.0",port=1121)
