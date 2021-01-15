import unittest
from mafiabot import *

class TestSimpleGames(unittest.TestCase):

  def assertDayPhasePlayers(self, mstate:MState, phase:MPhase, day:int, nplayers:int):
    self.assertEqual(mstate.phase,phase)
    self.assertEqual(mstate.day, day)
    self.assertEqual(len(mstate.players),nplayers)

  def test_3p_day1win(self):

    mstate = MState(1,MRules())
    mstate.start(['1','2','3'], ['TOWN','TOWN','MAFIA'],{})
    self.assertDayPhasePlayers(mstate,MPhase.DAY,1,3)

    mstate.vote('1','3')

    self.assertDayPhasePlayers(mstate,MPhase.DAY,1,3)

    self.assertRaises(TeamWinException, mstate.vote, '2','3')

  def test_3p_night1win(self):
    mstate = MState(1,MRules())
    mstate.start(['1','2','3'], ['TOWN','TOWN','MAFIA'],{})
    self.assertDayPhasePlayers(mstate,MPhase.DAY,1,3)

    mstate.vote('1','3')
    self.assertDayPhasePlayers(mstate,MPhase.DAY,1,3)
    mstate.vote('2','1')
    mstate.vote('3','2')
    mstate.vote('1',None)
    mstate.vote('1',NOTARGET)
    mstate.vote('2',None)
    mstate.vote('1','1')
    mstate.vote('3',NOTARGET)
    self.assertDayPhasePlayers(mstate,MPhase.DAY,1,3)
    mstate.vote('2',NOTARGET)
    self.assertDayPhasePlayers(mstate,MPhase.NIGHT,1,3)
    self.assertRaises(TeamWinException, mstate.mtarget, '3', '1')


if __name__ == '__main__':
  unittest.main()