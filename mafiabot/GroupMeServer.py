
from flask import Flask, request

class GroupMeServer:

  def __init__(self):
    app = Flask('mafiabot')

    app.add_url_rule('/', 'index', self.index, methods=['POST'])

    app.run()

  @staticmethod
  def index():
    print(request.form)
    return "ok"


  def serve(self, handle_chat, handle_dm):
    pass

if __name__ == '__main__':
  server = GroupMeServer()

  while True:
    pass
  
