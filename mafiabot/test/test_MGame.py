import unittest
import time
from .test_util import *
from ..mafiastate import EndGameException
from ..mafiactrl import MGame, FastMTimer

class Test_MGame(unittest.TestCase):

  def end_game(self, g_id, msg):
    self.end_game_flag = True
  
  def destroy_callback(self, g_id):
    self.destroy_callback_flag = True

  def tryEnd(self):
    self.assertTrue(self.end_game_flag)
    time.sleep(5)
    self.assertTrue(self.destroy_callback_flag)

  def setUp(self):
    self.mgame = MGame('1')
    self.mgame.end_game = self.end_game
    self.mgame.MTimerType = FastMTimer
    self.end_game_flag = False
    self.mgame.destroy_callback = self.destroy_callback
    self.destroy_callback_flag = False

  def test_mgame_simple(self):

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

    self.mgame.start(users,roleGen)
    self.assertTrue(self.mgame.handle_vote('1','2'))
    self.assertTrue(self.mgame.handle_vote('2','2'))
    self.assertTrue(self.mgame.handle_vote('3','2'))
    assertDayPhasePlayers(self, self.mgame.mstate, MPhase.NIGHT, 1, 4)
    self.assertTrue(self.mgame.handle_mtarget('5','1'))
    assertDayPhasePlayers(self, self.mgame.mstate, MPhase.DAY, 2, 3)
    self.assertTrue(self.mgame.handle_vote('4','3'))
    self.mgame.handle_vote('5','3')
    self.tryEnd()