
from mafiabot import *
from mafiabot.test.test_util import *


def default_roleGen(ids):
  default_roles = ["TOWN","TOWN","MAFIA"] * 5
  roles = default_roles[:len(ids)]
  assignments = list(zip(ids,roles))
  return (assignments,{})

class TestMGameSimple(unittest.TestCase):

  def test_first(self):

    mgame = MGame.new(MRules())

    mgame.start({'1':'ONE','2':'TWO','3':'THREE'}, default_roleGen)

    mgame.handle_vote('1','1')

    mgame.handle_vote('2','1')

    self.assertEqual(mgame.mstate.phase, MPhase.END)

    self.assertRaises(DeleteGameException, mgame.handle_end, '1')

    print("Done...")