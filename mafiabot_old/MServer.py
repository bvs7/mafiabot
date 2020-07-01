from .MState import MState
from .MEx import MPlayerID

import groupy
from flask import Flask, request
from typing import List, Tuple


# Setup globals
games : List[MState] = []
users : List[MPlayerID] = []

# Open groupme
with open("../groupme.key", 'r') as f:
  client = groupy.Client.from_token(f.read())

# initialize
# Initialize users
# Initialize saved mstates

app = Flask(__name__)

# A post to the lobby chat
@app.route('/lobby', methods=['POST'])
def lobby():
  # Ensure this was posted to lobby chat
  # Check to see if this is a request
  # If so, get details of request
  # process ReqType
  pass

# A post to a main chat
@app.route('/main', methods=['POST'])
def main():
  # Ensure this was posted to a main chat
  # Check to see if this is a request
  # If so, get details and route to Handler
  #  or process ourselves?
  pass

# A post to a mafia chat
@app.route('/mafia', methods=['POST'])
def mafia():
  # Ensure this was posted to a mafia chat
  # Check to see if this is a request
  # If so, get details and route to Handler
  pass

# A message in a dm
@app.route('/dm', methods=['POST'])
def dm():
  # Ensure this was a dm
  # Check to see if this is a request
  # If so, either:
  #  route to handler
  #  process ReqType
  pass

