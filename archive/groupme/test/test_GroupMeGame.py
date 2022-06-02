import unittest
import time
from .. import GroupMeGame
from ...chatinterface import TestMChat, TestMDM, MChat, MDM, MCmd
from ...test.test_util import *
from ...mafiactrl import FastMTimer

def create_vote_for(p_id):
  data = {
    'attachments':[
      {'type':'mentions',
      'user_ids':[p_id]}
    ]
  }
  return data

class Test_GroupMeGame(unittest.TestCase):

  def end_game(self, g_id, msg):
    self.end_game_flag = True
  
  def destroy_callback(self, g_id):
    self.destroy_callback_flag = True

  def tryEnd(self):
    self.assertTrue(self.end_game_flag)
    time.sleep(5)
    self.assertTrue(self.destroy_callback_flag)

  @classmethod
  def setUpClass(cls):
    cls.MGameType = GroupMeGame
    cls.MGameType.MChatType = MChat
    cls.MGameType.MDMType = MDM

  def setUp(self):
    self.mgame = self.MGameType('1')
    self.mgame.end_game = self.end_game
    self.mgame.MTimerType = FastMTimer
    self.end_game_flag = False
    self.mgame.destroy_callback = self.destroy_callback
    self.destroy_callback_flag = False

  def test_simple1(self):
    self.mgame.main_chat = TestMChat('MAIN', test_id=1)
    self.mgame.mafia_chat = TestMChat('MAFIA', self.mgame.main_chat, test_id=1)

    users = get_users(3)
    roleGen = get_roleGen(['TOWN','TOWN','MAFIA'])
    self.mgame.start(users,roleGen)

    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'1',MCmd.VOTE, \
        text="vote @TWO",data=create_vote_for('2')))

    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'2',MCmd.VOTE, \
        text="vote @ONE",data=create_vote_for('1')))
    
    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'3',MCmd.VOTE, \
        text="vote @TWO",data=create_vote_for('2')))

    self.tryEnd()

  def test_simple2(self):
    self.mgame.main_chat = TestMChat('MAIN', test_id=2)
    self.mgame.mafia_chat = TestMChat('MAFIA', self.mgame.main_chat, test_id=2)
    users = get_users(3)
    self.mgame.dms = TestMDM(self.mgame.main_chat, test_id=2, user_ids=list(users.keys()))

    roleGen = get_roleGen(['COP','TOWN','MAFIA'])
    self.mgame.start(users,roleGen)
    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'1',MCmd.VOTE, \
        text="vote nokill",data={}))

    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'2',MCmd.VOTE, \
        text="vote nokill",data={}))

    self.mgame.handle_dm('1',MCmd.TARGET, "target C")
    self.mgame.handle_chat('MAFIA','3',MCmd.TARGET,text="target D")

    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'1',MCmd.VOTE, \
        text="vote @TWO",data=create_vote_for('2')))

    self.assertTrue( 
      self.mgame.handle_chat("MAIN",'2',MCmd.VOTE, \
        text="vote me",data={}))
