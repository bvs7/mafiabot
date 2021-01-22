import unittest
from collections import deque

from mafiabot import MState, MRules, MPhase

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
  mstate = MState(rules,cast_main=print_mode,cast_mafia=print_mode,send_dm=print_mode)
  mstate.halt_timer = halt_timer
  return mstate

def assertDayPhasePlayers(testCase, mstate:MState, phase:MPhase, day:int, nplayers:int):
  testCase.assertEqual(mstate.phase,phase)
  testCase.assertEqual(mstate.day, day)
  testCase.assertEqual(len(mstate.players),nplayers)

def create_chat_tester(p_mode):
  nextMsg = deque([])
  def chat_tester(msg):
    if not len(nextMsg) == 0:
      m = nextMsg.popleft()
      if not msg == m:
        raise Exception(msg,m)
      else:
        msg = "Successful Chat!: " + msg
    p_mode(msg)
  def queue_to_chat(msg):
    nextMsg.append(msg)
  return chat_tester,queue_to_chat

def create_dm_tester(p_mode):
  nextMsgs = {}
  def dm_tester(msg, p_id):
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
  return dm_tester, add_dm