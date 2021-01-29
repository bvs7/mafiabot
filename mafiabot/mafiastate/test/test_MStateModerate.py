
from .. import *
from ..test.test_util import *

class TestMStateModerate(unittest.TestCase):

  def test_revenge_on_last_mafia(self):
    mstate = standardState()
    contract = MContract(MRole.IDIOT, '5', False)
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','IDIOT'])), {'5':contract})

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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','IDIOT'])), {'5':contract})

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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','IDIOT'])), {'5':contract})
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
    mstate.start(list(zip(['1','2','4','5'], ['TOWN','TOWN','MAFIA','IDIOT'])), {'5':contract})

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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','IDIOT','IDIOT'])), {'4':contract4,'5':contract5})

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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','IDIOT'])), {'5':contract})
    mstate.vote('1','4')
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as ege:
      mstate.vote('3','4')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='IDIOT',player='5'), ege.exception.msg)

  def test_survivor_win_town(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','SURVIVOR'])), {'5':contract})
    mstate.vote('1','4')
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('3','4')
    self.assertIn(resp_lib["CONTRACT_WIN"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)


  def test_survivor_win_mafia(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(list(zip(['1','2','4','5'], ['TOWN','TOWN','MAFIA','SURVIVOR'])), {'5':contract})
    mstate.mtarget('4','2')
    mstate.vote('5','1')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('4','1')
    self.assertIn(resp_lib["CONTRACT_WIN"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)

  def test_survivor_lose(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','SURVIVOR'])), {'5':contract})
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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','SURVIVOR'])), {'5':contract})
    mstate.vote('1','5')
    mstate.vote('2','5')
    with self.assertRaises(EndGameException) as iae:
      mstate.vote('3','5')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)

  def test_survivor_lose_last_kill(self):
    mstate = standardState()
    contract = MContract(MRole.SURVIVOR, '5', True)
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','SURVIVOR'])), {'5':contract})
    mstate.vote('1',NOTARGET)
    mstate.vote('2',NOTARGET)
    mstate.vote('3',NOTARGET)
    with self.assertRaises(EndGameException) as iae:
      mstate.mtarget('3','5')
    self.assertIn(resp_lib["CONTRACT_LOSE"].format(role='SURVIVOR',player='5'),
        iae.exception.msg)

  def test_boring_success(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','GUARD'])), {'5':contract})
    mstate.vote('1','4')
    mstate.vote('2','4')
    with self.assertRaises(EndGameException) as ege:
      mstate.vote('3','4')
    expected = resp_lib["CONTRACT_WIN"].format(role='GUARD',player='5') + " " + resp_lib["CHARGE_REVEAL"].format(charge='1')
    self.assertIn(expected, ege.exception.msg)

  def test_day2_success(self):
    mstate = standardState()
    contract = MContract(MRole.GUARD, '1', True)
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','GUARD'])), {'5':contract})
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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','MAFIA','MAFIA','GUARD'])), {'5':contract})
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
    mstate.start(list(zip(['1','2','3','4','5','6','7','8','9','10'],
      ['TOWN','TOWN','TOWN','TOWN','TOWN','MAFIA','MAFIA','MAFIA','AGENT','GUARD'])), {'9':contract2, '10':contract})
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
    mstate.start(list(zip(['1','2','3','4','5','6'],
      ['TOWN','TOWN','TOWN','MAFIA','AGENT','GUARD'])), {'5':contract2, '6':contract})
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
    contract_idiot = MContract(MRole.IDIOT, '1', False)
    contract_guard = MContract(MRole.GUARD, '2', True)
    mstate.start(list(zip(['1','2','3','4','5'],
      ['IDIOT','TOWN','TOWN','MAFIA','GUARD'])), {'1':contract_idiot, '5':contract_guard})
    
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
    mstate.start(list(zip(['1','2','3','4','5'], ['TOWN','TOWN','TOWN','MAFIA','GUARD'])), {'5':contract})

    mstate.vote('1','1')
    mstate.vote('4','1')

    expected = resp_lib["CHARGE_DIE_GUARD"].format(charge='1',aggressor='5')
    expected += "\n" + resp_lib["REFOCUS"].format(new_role="IDIOT")
    add_dm(expected, '5')

    mstate.vote('5','1')

  def test_large_game(self):
    mstate = standardState()
    ns = [str(n) for n in range(500)]
    roles = ['TOWN']*480 + ["MAFIA"]*20
    mstate.start(list(zip(ns,roles)),{})
