import unittest
import time
from .test_util import *
from ..mafiastate import EndGameException
from ..MSave import mafia_hook, MSaveEncoder, mload, msave
from ..mafiactrl import MGame, FastMTimer

class Test_MGameSaving(unittest.TestCase):

  def test_MGameSaving(self):

    mgame = MGame('1')

    with open("test_game.maf",'w') as f:
      msave(mgame, f)

    with open("test_game.maf", 'r') as f1:
      with open("test_game_exp1.maf", 'r') as f2:
        self.assertListEqual(f1.readlines(), f2.readlines())

    with open('test_game.maf','r') as ff:
      mgame2 = mload(ff)

    self.assertIsInstance(mgame2, MGame)

  def test_MGameStateSaving(self):
    
    mgame = MGame('2')

    users = {
      '1':'ONE',
      '2':'TWO',
      '3':'THREE',
      '4':'FOUR',
      '5':'FIVE',
    }

    def roleGen(ids):
      return (
        [('1','TOWN'), ('2','TOWN'),('3','TOWN'),('4','TOWN'),('5','MAFIA')],
        {}
      )

    mgame.start(users,roleGen)
    with open("test_game.maf",'w') as f:
      msave(mgame, f)
    mgame = None
    with open("test_game.maf",'r') as f:
      mgame = mload(f)

    assertDayPhasePlayers(self, mgame.mstate, MPhase.DAY, 1, 5)

    self.assertTrue(mgame.handle_vote('1','2'))
    self.assertTrue(mgame.handle_vote('2','2'))
    self.assertTrue(mgame.handle_vote('3','2'))

    assertDayPhasePlayers(self, mgame.mstate, MPhase.NIGHT, 1, 4)

    with open("test_game.maf",'w') as f:
      msave(mgame, f)
    mgame = None
    with open("test_game.maf",'r') as f:
      mgame = mload(f)

    assertDayPhasePlayers(self, mgame.mstate, MPhase.NIGHT, 1, 4)
    self.assertTrue(mgame.handle_mtarget('5','1'))
    assertDayPhasePlayers(self, mgame.mstate, MPhase.DAY, 2, 3)

    with open("test_game.maf",'w') as f:
      msave(mgame, f)
    mgame = None
    with open("test_game.maf",'r') as f:
      mgame = mload(f)

    assertDayPhasePlayers(self, mgame.mstate, MPhase.DAY, 2, 3)

    self.assertTrue(mgame.handle_vote('4','3'))

    self.end_game_flag = False
    self.destroy_callback_flag = False

    def end_game(g_id,msg):
      self.end_game_flag = True
    
    def destroy_callback(g_id):
      self.destroy_callback_flag = True

    mgame.end_game = end_game
    mgame.destroy_callback = destroy_callback

    mgame.MTimerType = FastMTimer

    mgame.handle_vote('5','3')

    self.assertTrue(self.end_game_flag)
    self.assertFalse(self.destroy_callback_flag)

    time.sleep(5) # Give game time to destroy (using FastMTimer)

    self.assertTrue(self.destroy_callback_flag)