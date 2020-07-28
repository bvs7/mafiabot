#!/usr/bin/python3
from flask import Flask, request
import json

class GroupMeServer:

  def __init__(self):
    app = Flask('mafiabot')

    app.add_url_rule('/', 'chat', self.chat, methods=['POST'])
    app.add_url_rule('/dm', 'dm', self.dm, methods=['POST'])

    app.run(host="0.0.0.0",port=1121)

  @staticmethod
  def chat():
    data = json.loads(request.data.decode('utf-8'))
    print("Chat:")
    print(data)
    return "ok"

  @staticmethod
  def dm():
    data = json.loads(request.data.decode('utf-8'))
    print("DM:")
    print(data)
    return "ok"

if __name__ == '__main__':
  server = GroupMeServer()

  while True:
    pass
