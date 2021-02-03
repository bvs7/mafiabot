import unittest
from collections import deque

from ..chatinterface import MChat, MDM
from ..mafiastate import MState, MRules, MPhase

def verbose(*args):
  print(*args)

def chunk(*args):
  print(args)

def silent(*args):
  pass

#print_mode = verbose
#print_mode = chunk
print_mode = silent

def halt_timer(*args):
  print_mode("Halt Timer!")

def standardState(rules=MRules()):
  mstate = MState(rules=rules)
  mstate.halt_timer = halt_timer
  return mstate

def assertDayPhasePlayers(testCase, mstate:MState, phase:MPhase, day:int, nplayers:int):
  testCase.assertEqual(mstate.phase,phase)
  testCase.assertEqual(mstate.day, day)
  testCase.assertEqual(len(mstate.players),nplayers)

def create_chat_tester(p_mode):
  nextMsg = deque([])
  class ChatTester(MChat):
    def chat(self, msg):
      if not len(nextMsg) == 0:
        m = nextMsg.popleft()
        if not msg == m:
          raise Exception(msg,m)
        else:
          msg = "Successful Chat!: " + msg
      p_mode(msg)
  def queue_to_chat(msg):
    nextMsg.append(msg)
  return ChatTester(),queue_to_chat

def create_dm_tester(p_mode):
  nextMsgs = {}
  class DMTester(MDM):
    def send(self, msg, p_id):
      if p_id in nextMsgs and not len(nextMsgs[p_id]) == 0:
        m = nextMsgs[p_id].popleft()
        if not m ==  msg:
          raise Exception(msg,m)
        else:
          msg = "Successful DM! [{}]: ".format(p_id) + msg
      p_mode(msg,p_id)
  def add_dm(msg, p_id):
    if not p_id in nextMsgs:
      nextMsgs[p_id] = deque([])
    nextMsgs[p_id].append(msg)
  return DMTester(), add_dm

def create_handle_chat_tester(p_mode):
  nextCmd = deque()
  def handle_chat(group_id, sender_id, cmd, **kwargs):
    m = nextCmd.popleft()
    kwargs["group_id"] = group_id
    kwargs["sender_id"] = sender_id
    kwargs["cmd"] = cmd
    for k in m:
      if not (k in kwargs and m[k] == kwargs[k]):
        raise Exception(m,kwargs)
    p_mode(kwargs)
  def add_chat(group_id, sender_id, cmd, **kwargs):
    kwargs["group_id"] = group_id
    kwargs["sender_id"] = sender_id
    kwargs["cmd"] = cmd
    nextCmd.append(kwargs)
  return handle_chat, add_chat

def create_handle_dm_tester(p_mode):
  nextCmd = deque()
  def handle_dm(sender_id, cmd, **kwargs):
    m = nextCmd.popleft()
    kwargs["sender_id"] = sender_id
    kwargs["cmd"] = cmd
    for k in m:
      if not (k in kwargs and m[k] == kwargs[k]):
        raise Exception(m,kwargs)
    p_mode(kwargs)
  def add_dm(sender_id, cmd, **kwargs):
    kwargs["sender_id"] = sender_id
    kwargs["cmd"] = cmd
    nextCmd.append(kwargs)
  return handle_dm, add_dm