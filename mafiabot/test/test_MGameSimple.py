from mafiabot import MGame, MState, MRules

import unittest

def default_roleGen(ids):
  default_roles = ["TOWN","TOWN","MAFIA"] * 5
  roles = default_roles[:len(ids)]
  assignments = list(zip(ids,roles))
  return (assignments,{})

class TestMGameSimple(unittest.TestCase):

  def test_first(self):

    mstate = MState(1)

    mgame = MGame(None, mstate, "main chat", "mafia_chat")

    mgame.start({'1':'ONE','2':'TWO','3':'THREE'}, default_roleGen)

    mgame.handle_vote('1','1')
    mgame.handle_vote('2','1')