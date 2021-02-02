import unittest
from ... import test_util
from .. import MServer, TestMServer

class Test_TestMServer(unittest.TestCase):

  def setUp(self):
    self.handle_chat, self.add_chat = \
      test_util.create_handle_chat_tester(test_util.print_mode)
    self.handle_dm, self.add_dm = \
      test_util.create_handle_dm_tester(test_util.print_mode)

  def test_TestMServer(self):
    t = TestMServer(self.handle_chat, self.handle_dm)

    self.add_chat("LOBBY", '1','status',text="/status",data={})
    t.parse("/status LOBBY 1")

    self.add_chat("LOBBY", '1','status')
    t.parse("/status LOBBY 1")

    self.add_dm("1",'status', text="/status", data={})
    t.parse("dm /status 1")

  def test_TestMServer_file(self):
    t = TestMServer.from_file(self.handle_chat, self.handle_dm, "test_data/mserver0.in")
    
    self.add_chat("LOBBY",'1','status')
    self.add_chat("LOBBY",'2','help')
    self.add_chat('LOBBY','3','start')
    self.add_chat('MAIN','1','status')

    self.add_dm('3','help')
    self.add_dm('4','help')

    self.add_chat('MAFIA', '4', 'status')

    t.run()
