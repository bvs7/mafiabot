from .. import MContract, MRole, InvalidActionException, EndGameException
from ...test.test_util import * # pylint: disable=W0614

class TestMStateTimer(unittest.TestCase):

  def test_timer1(self):
    mstate = standardState()
    users = get_users(7)
    roleGen = get_roleGen(['TOWN','COP','DOCTOR','CELEB','STRIPPER','GOON','IDIOT'], 
      {'7':MContract(MRole.IDIOT, '7', False)})
    assignments,contracts = roleGen(users.keys())
    mstate.start(assignments,contracts)
    assertDayPhasePlayers(self, mstate, MPhase.DAY, 1, 7)
    mstate.timer()
    assertDayPhasePlayers(self, mstate, MPhase.NIGHT, 1, 7)
    mstate.timer()
    assertDayPhasePlayers(self, mstate, MPhase.DAY, 2, 7)
    mstate.vote('1','2')
    mstate.vote('3','2')
    mstate.timer()
    assertDayPhasePlayers(self, mstate, MPhase.NIGHT, 2, 7)
    mstate.mtarget('6','1')
    mstate.timer()
    assertDayPhasePlayers(self, mstate, MPhase.DAY, 3, 6)
    mstate.vote('5','2')
    mstate.vote('6','2')
    mstate.vote('6',None)
    mstate.vote('6','7')
    mstate.vote('2','7')
    mstate.vote('7','7')
    mstate.vote('5','7')
    assertDayPhasePlayers(self, mstate, MPhase.DUSK, 3, 6)
    mstate.timer()
    assertDayPhasePlayers(self, mstate, MPhase.NIGHT, 3, 4)
    try:
      mstate.mtarget('6','2')
      self.assertFalse(True)
    except InvalidActionException as iae:
      print(iae.msg, '6')

    mstate.target('2','6')
    mstate.timer()
    assertDayPhasePlayers(self, mstate, MPhase.DAY, 4, 4)
    mstate.reveal('4')
    mstate.vote('2','6')
    mstate.vote('3','2')
    mstate.vote('4','2')
    mstate.vote('6','2')
    assertDayPhasePlayers(self, mstate, MPhase.NIGHT, 4, 3)
    mstate.target('3','3')
    mstate.timer()
    mstate.reveal('4')
    mstate.vote('3','6')
    mstate.vote('6','3')
    with self.assertRaises(EndGameException):
      mstate.vote('4','6')


    
