import unittest
from collections import deque
from mafiabot import *

def verbose(*args):
  print(*args)

def chunk(*args):
  print(args)

def silent(*args):
  pass

print_mode = verbose
#print_mode = chunk
#print_mode = silent

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

class TestRoleIntro(unittest.TestCase):

  def all_roles(self, rules=MRules()):
    test_dm, add_dm = create_dm_tester(print_mode)
    mstate = standardState(rules)
    mstate.send_dm = test_dm
    ns = [str(n) for n in range(len(MRole.__members__))]
    roles = list(MRole.__members__.values())
    idiot_id = str(roles.index(MRole.IDIOT))
    survivor_id = str(roles.index(MRole.SURVIVOR))
    guard_id = str(roles.index(MRole.GUARD))
    agent_id = str(roles.index(MRole.AGENT))

    idiot_contract = MContract(MRole.IDIOT,idiot_id, False)
    survivor_contract = MContract(MRole.SURVIVOR, survivor_id, True)
    guard_contract = MContract(MRole.GUARD, '1', True)
    agent_contract = MContract(MRole.AGENT, '1', False)
    contracts = {
      idiot_id:idiot_contract,
      survivor_id:survivor_contract,
      guard_id:guard_contract,
      agent_id:agent_contract,
    }
    mstate.start(ns, roles, contracts)

  def test_many_rules(self):
    rules = MRules()
    self.all_roles(rules)

    rules[MRules.know_if_saved] = "SECRET"
    rules[MRules.know_if_saved_doc] = "ON"
    rules[MRules.know_if_saved_self] = "OFF"
    rules[MRules.charge_refocus_guard] = "DIE"
    rules[MRules.charge_refocus_agent] = "WIN"
    rules[MRules.idiot_vengeance] = "STUN"
    rules[MRules.know_if_stripped] = "ON"
    rules[MRules.no_milk_self] = "OFF"
    rules[MRules.cop_strength] = "ROLE"
    rules[MRules.unique_night_act] = "OFF"
    rules[MRules.goon_potence] = "OFF"
    print('======================')
    self.all_roles(rules)
    

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

  def test_mason(self):
    test_dm, add_dm = create_dm_tester(print_mode)
    mstate = standardState()
    mstate.send_dm = test_dm
    expected1 = MRole.MASON.expl()
    add_dm(expected1,'3')
    add_dm(expected1,'4')

    mstate.start(
      ['1','2','3','4','5'], 
      ['TOWN','MASON','MASON','MASON','MAFIA'], 
      {})


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

class TestIdiotSimpleRules(unittest.TestCase):

  def test_revenge_on_last_mafia(self):
    mstate = standardState()
    contract = MContract(MRole.IDIOT, '5', False)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','IDIOT'], {'5':contract})

    with self.assertRaises(InvalidActionException) as iae:
      mstate.itarget('5','2')
    self.assertEqual(iae.exception.msg, resp_lib["INVALID_ITARGET_PHASE"])

    mstate.vote('4','5')
    mstate.vote('5','5')
    mstate.vote('1','5')

    assertDayPhasePlayers(self,mstate,MPhase.DUSK, 1, 5)

    with self.assertRaises(InvalidActionException) as iae:
      mstate.itarget('4','5')
    self.assertEqual(iae.exception.msg, resp_lib["INVALID_ITARGET_PLAYER"])

    with self.assertRaises(InvalidActionException) as iae:
      mstate.itarget('5','2')
    self.assertEqual(iae.exception.msg, resp_lib["INVALID_ITARGETED"])

    with self.assertRaises(EndGameException) as iae:
      mstate.itarget('5','4')
    self.assertIn(resp_lib['CONTRACT_WIN'].format(role='IDIOT',player='5'),
      iae.exception.msg)


  def test_revenge_on_town(self):
    mstate = standardState()
    contract = MContract(MRole.IDIOT, '5', False)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','IDIOT'], {'5':contract})

    mstate.vote('4','5')
    mstate.vote('5','5')
    mstate.vote('1','5')
    mstate.itarget('5','1')

    assertDayPhasePlayers(self,mstate,MPhase.NIGHT, 1, 3)
    with self.assertRaises(EndGameException) as iae:
      mstate.mtarget('4','2')
    self.assertIn(resp_lib['CONTRACT_WIN'].format(role='IDIOT',player='5'),
      iae.exception.msg)

  def test_revenge_on_last_town(self):
    mstate = standardState()
    contract = MContract(MRole.IDIOT, '5', False)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','IDIOT'], {'5':contract})
    mstate.vote('4','5')
    mstate.vote('5','5')
    mstate.vote('1','5')
    
    assertDayPhasePlayers(self,mstate,MPhase.DUSK, 1, 5)
    with self.assertRaises(EndGameException) as iae:
      mstate.itarget('5','1')
    self.assertIn(resp_lib['CONTRACT_WIN'].format(role='IDIOT',player='5'),
      iae.exception.msg)

  def test_lose_to_kill(self):
    mstate = standardState()
    contract = MContract(MRole.IDIOT, '5', False)
    mstate.start(['1','2','4','5'], ['TOWN','TOWN','MAFIA','IDIOT'], {'5':contract})

    mstate.mtarget('4','5')
    mstate.vote('1','4')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('2','4')
    self.assertIn(resp_lib['CONTRACT_LOSE'].format(role='IDIOT',player='5'),
      iae.exception.msg)

  def test_lose_to_vengeance(self):
    mstate = standardState()
    contract4 = MContract(MRole.IDIOT, '4', False)
    contract5 = MContract(MRole.IDIOT, '5', False)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','IDIOT','IDIOT'], {'4':contract4,'5':contract5})

    mstate.vote('1','4')
    mstate.vote('2','4')
    mstate.vote('5','4')

    assertDayPhasePlayers(self,mstate,MPhase.DUSK,1,5)

    with self.assertRaises(InvalidActionException) as iae:
      mstate.itarget('5','1')
    self.assertEqual(resp_lib["INVALID_ITARGET_PLAYER"], iae.exception.msg)

    mstate.itarget('4','5')

    with self.assertRaises(EndGameException) as ege:
      mstate.mtarget('3','1')
    self.assertIn(resp_lib["CONTRACT_WIN"].format(role='IDIOT',player='4'), ege.exception.msg)
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='IDIOT',player='5'), ege.exception.msg)

  def test_lose_to_survive(self):
    mstate = standardState()
    contract = MContract(MRole.IDIOT, '5', False)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','IDIOT'], {'5':contract})
    mstate.vote('1','4')
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as ege:
      mstate.vote('3','4')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='IDIOT',player='5'), ege.exception.msg)

class TestSurvivorSimpleRules(unittest.TestCase):
  def test_survivor_win_town(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','SURVIVOR'], {'5':contract})
    mstate.vote('1','4')
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('3','4')
    self.assertIn(resp_lib["CONTRACT_WIN"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)


  def test_survivor_win_mafia(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(['1','2','4','5'], ['TOWN','TOWN','MAFIA','SURVIVOR'], {'5':contract})
    mstate.mtarget('4','2')
    mstate.vote('5','1')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('4','1')
    self.assertIn(resp_lib["CONTRACT_WIN"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)

  def test_survivor_lose(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','SURVIVOR'], {'5':contract})
    mstate.vote('1','5')
    mstate.vote('2','5')
    mstate.vote('3','5')
    mstate.mtarget('4','3')
    assertDayPhasePlayers(self,mstate,MPhase.DAY, 2, 3)
    mstate.vote('2','1')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('4','1')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)


  def test_survivor_lose_last_elect(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','SURVIVOR'], {'5':contract})
    mstate.vote('1','5')
    mstate.vote('2','5')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('3','5')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)

  def test_survivor_lose_last_kill(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','SURVIVOR'], {'5':contract})
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)
    with self.assertRaises(EndGameException) as iae:
      mstate.mtarget('3','5')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)

class TestGuardAgent(unittest.TestCase):
  def test_boring_success(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','GUARD'], {'5':contract})
    mstate.vote('1','4')
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as ege:
      mstate.vote('3','4')
    expected = resp_lib["CONTRACT_WIN"].format(role='GUARD',player='5') + " " + resp_lib["CHARGE_REVEAL"].format(charge='1')
    self.assertIn(expected, ege.exception.msg)

  def test_day2_success(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','GUARD'], {'5':contract})
    mstate.vote('1','2')
    mstate.vote('2','2')
    mstate.vote('3','2')
    mstate.mtarget('4','3')
    mstate.vote('5','4')
    with self.assertRaises(EndGameException) as ege:
      mstate.vote('1','4')
    expected = resp_lib["CONTRACT_WIN"].format(role='GUARD',player='5') + " " + resp_lib["CHARGE_REVEAL"].format(charge='1')
    self.assertIn(expected, ege.exception.msg)

  def test_night_fail(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','GUARD'], {'5':contract})
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)
    with self.assertRaises(EndGameException) as ege:
      mstate.mtarget('4','1')
    expected = resp_lib["CONTRACT_LOSE"].format(role='AGENT',player='5') + " " + resp_lib["CHARGE_REVEAL"].format(charge='4')
    self.assertIn(expected, ege.exception.msg)

  def test_refocus(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    contract2 = MContract(MRole.AGENT, '1', False)
    mstate.start(['1','2','3','4','5','6','7','8','9','10'],
      ['TOWN','TOWN','TOWN','TOWN','TOWN','MAFIA','MAFIA','MAFIA','AGENT','GUARD'], {'9':contract2, '10':contract})
    mstate.mtarget('6','1')
    self.assertEqual(mstate.players['9'].role, MRole.GUARD)
    self.assertEqual(mstate.players['10'].role, MRole.AGENT)
    self.assertEqual(mstate.contracts['9'].charge, '6')
    self.assertEqual(mstate.contracts['10'].charge, '6')

    mstate.vote('10','6')
    mstate.vote('3','6')
    mstate.vote('4','6')
    mstate.vote('7','6')
    mstate.vote('2','6')

    self.assertEqual(mstate.players['10'].role, MRole.GUARD)
    self.assertEqual(mstate.players['9'].role, MRole.AGENT)
    self.assertEqual(mstate.contracts['9'].charge, '2')
    self.assertEqual(mstate.contracts['10'].charge, '2')

    mstate.mtarget('7','2')
    
    self.assertEqual(mstate.players['9'].role, MRole.GUARD)
    self.assertEqual(mstate.players['10'].role, MRole.AGENT)
    self.assertEqual(mstate.contracts['9'].charge, '7')
    self.assertEqual(mstate.contracts['10'].charge, '7')

    mstate.vote('3','7')
    mstate.vote('4','7')
    mstate.vote('7','7')
    mstate.vote('10','7')

    self.assertEqual(mstate.players['9'].role, MRole.AGENT)
    self.assertEqual(mstate.players['10'].role, MRole.SURVIVOR)
    self.assertEqual(mstate.contracts['9'].charge, '10')
    self.assertEqual(mstate.contracts['10'].charge, '10')

    mstate.mtarget('8','3')

    mstate.vote('9','10')
    mstate.vote('8','10')
    mstate.vote('4','10')

    self.assertEqual(mstate.contracts['10'].success, False)
    self.assertEqual(mstate.players['9'].role, MRole.GUARD)
    self.assertEqual(mstate.contracts['9'].charge, '4')

    mstate.mtarget('8','9')

    self.assertEqual(mstate.contracts['9'].success, True)

    mstate.vote('5','4')

    with self.assertRaises(EndGameException) as ege:
      mstate.vote('8','4')
    expected1 = resp_lib['CONTRACT_LOSE'].format(role='SURVIVOR',player='10')
    expected2 = resp_lib['CONTRACT_LOSE'].format(role='GUARD',player='9') + " " +resp_lib["CHARGE_REVEAL"].format(charge='4')
    self.assertIn(expected1, ege.exception.msg)
    self.assertIn(expected2, ege.exception.msg)
    
  def test_postmortem_change(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    contract2 = MContract(MRole.AGENT, '1', False)
    mstate.start(['1','2','3','4','5','6'],
      ['TOWN','TOWN','TOWN','MAFIA','AGENT','GUARD'], {'5':contract2, '6':contract})
    mstate.mtarget('4','5')
    mstate.vote('1','6')
    mstate.vote('2','6')
    mstate.vote('4','6')
    self.assertEqual(mstate.contracts['5'].success, False)
    self.assertEqual(mstate.contracts['6'].success, True)
    mstate.mtarget('4','1')
    self.assertEqual(mstate.contracts['5'].success, True)
    self.assertEqual(mstate.contracts['6'].success, False)
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as ege:
      mstate.vote('3','4')
    expected1 = resp_lib["CONTRACT_LOSE"].format(role='GUARD',player='6') + " " + resp_lib["CHARGE_REVEAL"].format(charge='1')
    expected2 = resp_lib["CONTRACT_WIN"].format(role='AGENT',player='5') + " " + resp_lib["CHARGE_REVEAL"].format(charge='1')
    self.assertIn(expected1, ege.exception.msg)
    self.assertIn(expected2, ege.exception.msg)

  def test_idiot_vengeance_wow(self):
    mstate = standardState()
    dm_tester, add_dm = create_dm_tester(print_mode)
    mstate.send_dm = dm_tester
    contract_idiot = MContract(MRole.IDIOT, '1', True)
    contract_guard = MContract(MRole.GUARD, '2', False)
    mstate.start(['1','2','3','4','5'],
      ['IDIOT','TOWN','TOWN','MAFIA','GUARD'], {'1':contract_idiot, '5':contract_guard})
    
    # 2 votes for idiot (last)
    mstate.vote('1','1')
    mstate.vote('3','1')
    mstate.vote('2','1')
    # idiot takes revenge on 2

    expected = resp_lib["CHARGE_DIE_GUARD"].format(charge='2',aggressor='1')
    expected += "\n" + resp_lib["REFOCUS"].format(new_role="AGENT")
    expected += "\n" + resp_lib["CHARGE_ASSIGN"].format(charge='1')
    add_dm(expected, '5')
    expected = resp_lib["CHARGE_DIE_AGENT"].format(charge='1',aggressor='2')
    expected += "\n" + resp_lib["REFOCUS"].format(new_role="SURVIVOR")
    add_dm(expected,'5')

    mstate.itarget('1','2')

  def test_refocus_to_idiot(self):
    mstate = standardState()
    dm_tester, add_dm = create_dm_tester(print_mode)
    mstate.send_dm = dm_tester
    contract = MContract(MRole.GUARD, '1', True)
    mstate.start(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','GUARD'], {'5':contract})

    mstate.vote('1','1')
    mstate.vote('4','1')

    expected = resp_lib["CHARGE_DIE_GUARD"].format(charge='1',aggressor='5')
    expected += "\n" + resp_lib["REFOCUS"].format(new_role="IDIOT")
    add_dm(expected, '5')

    mstate.vote('5','1')


if __name__ == '__main__':
  unittest.main()