#!/usr/bin/python3
from flask import Flask, request
import json


class GroupMeServer:

  def __init__(self):
    app = Flask('mafiabot')

    app.add_url_rule('/', 'index', self.index, methods=['POST'])

    app.run(host="0.0.0.0",port=1121)

  @staticmethod
  def index():
    data = json.loads(request.data.decode('utf-8'))
    print(data)
    return "ok"


  def serve(self, handle_chat, handle_dm):
    pass

if __name__ == '__main__':
  server = GroupMeServer()

  while True:
    pass
