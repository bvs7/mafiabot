import unittest
from collections import deque
from mafiabot import *

def verbose(*args):
  print(*args)

def chunk(*args):
  print(args)

def silent(*args):
  pass

print_mode = chunk

def standardState(rules=MRules()):
  return MState(1,rules,cast_main=print_mode,cast_mafia=print_mode,send_dm=print_mode)

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

class TestStandardRuleSimpleGames(unittest.TestCase):

  def test_3p_day1win(self):

    mstate = standardState()
    mstate.start(['1','2','3'], ['TOWN','TOWN','MAFIA'],{})
    assertDayPhasePlayers(self,mstate,MPhase.DAY,1,3)

    mstate.vote('1','3')

    assertDayPhasePlayers(self,mstate,MPhase.DAY,1,3)

    self.assertRaises(TeamWinException, mstate.vote, '2','3')

  def test_3p_night1win(self):
    mstate = standardState()
    mstate.start(['1','2','3'], ['TOWN','TOWN','MAFIA'],{})
    assertDayPhasePlayers(self,mstate,MPhase.DAY,1,3)

    mstate.vote('1','3')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,1,3)
    mstate.vote('2','1')
    mstate.vote('3','2')
    mstate.vote('1',None)
    mstate.vote('1',NOTARGET)
    mstate.vote('2',None)
    mstate.vote('1','1')
    mstate.vote('3',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.DAY,1,3)
    mstate.vote('2',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,3)
    self.assertRaises(TeamWinException, mstate.mtarget, '3', '1')

  def test_4p_nokills(self):
    mstate = standardState()
    mstate.start(['1','2','3','4'], ['TOWN','TOWN','MAFIA','TOWN'],{})
    # start night
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,4)
    mstate.mtarget('3',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,4)
    mstate.vote('1','2')
    mstate.vote('2','2')
    mstate.vote('3','4')
    mstate.vote('4','4')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,4)
    mstate.vote('2',NOTARGET)
    mstate.vote('4',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,4)
    self.assertRaises(TeamWinException,
      mstate.mtarget, '3', '3')

  def test_10p_mafiawin(self):
    mstate = standardState()
    mstate.start(['1','2','3','4','5','6','7','8','9','10'], ['TOWN']*7 + ['MAFIA']*3, {})
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,10)
    mstate.mtarget('8','1')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,9)
    mstate.vote('2','2')
    mstate.vote('3','2')
    mstate.vote('4','2')
    mstate.vote('5','2')
    mstate.vote('6','2')
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,8)
    mstate.mtarget('8','3')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,3,7)
    mstate.vote('4','4')
    mstate.vote('5','4')
    mstate.vote('6','4')
    self.assertRaises(TeamWinException,
      mstate.vote, '7','4')

  def test_10p_townwin(self):
    mstate = standardState()
    mstate.start(['1','2','3','4','5','6','7','8','9','10'], ['TOWN']*7 + ['MAFIA']*3, {})
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,10)
    mstate.mtarget('10','9')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,9)
    mstate.vote('1','8')
    mstate.vote('2','8')
    mstate.vote('3','8')
    mstate.vote('4','8')
    mstate.vote('6','10')
    mstate.vote('5','8')
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,8)
    mstate.mtarget('10','7')
    mstate.vote('1','10')
    mstate.vote('2','10')
    mstate.vote('3','10')
    self.assertRaises(TeamWinException,
      mstate.vote,'4','10')
  
  def test_invalid_votes(self):
    mstate = standardState()
    mstate.start(['1','2','3','4'], ['TOWN']*3 + ['MAFIA'], {})

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,4)

    try:
      mstate.vote('1','2')
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_VOTE_PHASE"])

    mstate.mtarget('4','3')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,3)

    try:
      mstate.vote('1','3')
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_VOTEE"].format(player_id='3'))

    try:
      mstate.vote('3','3')
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_VOTER"].format(player_id='3'))

    try:
      mstate.vote('1','Hello')
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_VOTEE"].format(player_id='Hello'))

    try:
      mstate.vote('3',NOTARGET)
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_VOTER"].format(player_id='3'))

    mstate.vote('2', NOTARGET)
    mstate.vote('4', NOTARGET)
    
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,3)

    try:
      mstate.vote('1','4')
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_VOTE_PHASE"])

class TestStandardRuleModerateRoles(unittest.TestCase):

  def test_cop(self):
    test_dm, add_dm = create_dm_tester(print_mode)
    mstate = standardState()
    mstate.send_dm = test_dm
    mstate.start(['1','2','3','4','5'], ['COP', 'TOWN','MILLER','MAFIA','GODFATHER'], {})
    assertDayPhasePlayers(self,mstate,MPhase.DAY,1,5)
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,5)
    mstate.mtarget('5',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,5)
    add_dm(resp_lib["TARGET"].format(target='2'), '1')
    add_dm(resp_lib["INVESTIGATE"].format(target='2',role='Not Mafia Aligned'), '1')
    mstate.target('1','2')
    
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,5)
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,5)
    add_dm(resp_lib["TARGET"].format(target='3'), '1')
    add_dm(resp_lib["INVESTIGATE"].format(target='3',role='Mafia Aligned'), '1')
    mstate.target('1','3')
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,5)
    mstate.mtarget('5',NOTARGET)

    assertDayPhasePlayers(self,mstate,MPhase.DAY,3,5)
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,3,5)
    mstate.mtarget('5','1')
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,3,5)
    mstate.mtarget('5',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,3,5)
    add_dm(resp_lib["TARGET"].format(target='4'), '1')
    add_dm(resp_lib["INVESTIGATE"].format(target='4',role='Mafia Aligned'), '1')
    mstate.target('1','4')

    assertDayPhasePlayers(self,mstate,MPhase.DAY,4,5)
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,4,5)
    mstate.mtarget('5',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,4,5)
    mstate.mtarget('4','1')
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,4,5)
    mstate.mtarget('5',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,4,5)
    with self.assertRaises(InvalidActionException) as iae:
      mstate.target('1','7')
      self.assertEqual(iae.exception.msg, resp_lib["INVALID_TARGETED"].format(target_id='7'))
      
    add_dm(resp_lib["TARGET"].format(target='5'), '1')
    add_dm(resp_lib["INVESTIGATE"].format(target='5',role='Not Mafia Aligned'), '1')
    mstate.target('1','5')

    assertDayPhasePlayers(self,mstate,MPhase.DAY,5,5)
    mstate.vote('1','5')
    mstate.vote('2','5')
    mstate.vote('3','5')

    mstate.mtarget('4','3')
    add_dm(resp_lib["TARGET"].format(target='3'), '1')
    add_dm(resp_lib["INVESTIGATE"].format(target='3',role='Not Mafia Aligned'), '1')
    mstate.target('1','3')

    assertDayPhasePlayers(self,mstate,MPhase.DAY,6,3)
    mstate.vote('1','4')
    self.assertRaises(TeamWinException, mstate.vote,'2','4')

  def test_doctor(self):
    mstate = standardState()
    mstate.start(['1','2','3','4'], ['DOCTOR', 'TOWN','TOWN','MAFIA'], {})

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,4)

    mstate.mtarget('4', '1')
    mstate.target('1','1')
    
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,4)

    mstate.vote('1','3')
    mstate.vote('2','3')
    mstate.vote('4','3')

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,3)
    mstate.target('1','1')
    mstate.target('1','2')
    mstate.mtarget('4','2')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,3,3)
    
    mstate.vote('1','4')
    self.assertRaises(TeamWinException, mstate.vote,'2','4')

  def test_celeb(self):
    c_main, add_to_c_main = create_chat_tester(print_mode)
    s_dm, add_dm = create_dm_tester(print_mode)
    mstate = standardState()
    mstate.cast_main = c_main
    mstate.send_dm = s_dm
    mstate.start(['1','2','3','4'], ['CELEB', 'CELEB','TOWN','STRIPPER'], {})

    try:
      mstate.reveal('1')
      self.assertFalse(True, "Shouldn't get here!")
    except InvalidActionException as e:
      self.assertEqual(e.msg, resp_lib["INVALID_REVEAL_PHASE"])

    mstate.mtarget('4','3')
    mstate.target('4','1')

    add_dm(resp_lib["STRIPPED"], '1')
    mstate.reveal('1')
    add_dm(resp_lib["STRIPPED"], '1')
    mstate.reveal('1')
    self.assertRaises(InvalidActionException, mstate.reveal, '3')
    self.assertRaises(InvalidActionException, mstate.reveal, '4')
    
    add_to_c_main(resp_lib["REVEAL"].format(actor='2',role='CELEB'))
    mstate.reveal('2')

    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.mtarget('4',NOTARGET)
    mstate.target('4','2')

    add_to_c_main(resp_lib["REVEAL_REMINDER"].format(actor='2',role='CELEB'))
    mstate.reveal('2')
    add_to_c_main(resp_lib["REVEAL_REMINDER"].format(actor='2',role='CELEB'))
    mstate.reveal('2')
    add_to_c_main(resp_lib["REVEAL"].format(actor='1',role='CELEB'))
    mstate.reveal('1')

  def test_goon(self):
    test_dm, add_dm = create_dm_tester(print_mode)
    mstate = standardState()
    mstate.send_dm = test_dm
    add_dm(resp_lib["STUN"], '4')
    mstate.start(['1','2','3','4','5','6'], ['TOWN', 'TOWN','TOWN','GOON', 'TOWN','TOWN'], {})
    with self.assertRaises(InvalidActionException) as iae:
      mstate.mtarget('4','1')
      self.assertEqual(iae.exception.msg, resp_lib["INVALID_TARGET_STUNNED"])
    mstate.mtarget('4',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,6)

    mstate.vote('1',NOTARGET)
    mstate.vote('4',NOTARGET)
    mstate.vote('3',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,2,6)
    mstate.mtarget('4','5')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,3,5)

    mstate.vote('1','3')
    mstate.vote('2','3')
    add_dm(resp_lib["STUN"], '4')
    mstate.vote('4','3')
    with self.assertRaises(InvalidActionException) as iae:
      mstate.mtarget('4','1')
      self.assertEqual(iae.exception.msg, resp_lib["INVALID_TARGET_STUNNED"])
    mstate.mtarget('4',NOTARGET)

    assertDayPhasePlayers(self,mstate,MPhase.DAY,4,4)

  def test_milky(self):
    mstate = standardState()
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','MILKY','MAFIA','TOWN'], {})
    mstate.vote('5',NOTARGET)
    mstate.vote('1',NOTARGET)
    mstate.vote('4',NOTARGET)
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,5)
    mstate.mtarget('4','5')
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,5)
    with self.assertRaises(InvalidActionException) as iae:
      mstate.target('3','3')
      self.assertEqual(iae.exception.msg, resp_lib["INVALID_TARGET_MILK_SELF"])
    assertDayPhasePlayers(self,mstate,MPhase.NIGHT,1,5)
    mstate.target('3','1')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,2,4)
    mstate.vote('3',NOTARGET)
    mstate.vote('4',NOTARGET)
    mstate.mtarget('4','1')
    mstate.target('3','1')
    assertDayPhasePlayers(self,mstate,MPhase.DAY,3,3)
    mstate.vote('3',NOTARGET)
    mstate.vote('4',NOTARGET)
    mstate.mtarget('4','3')
    with self.assertRaises(EndGameException):
      mstate.target('3','2')


if __name__ == '__main__':
  unittest.main()