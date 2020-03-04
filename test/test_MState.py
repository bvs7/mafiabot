import unittest
import sys
import inspect

sys.path.append("C:\\Users\\Omniscient\\Documents\\Personal\\MafiaBot")
from mafiabot import MRespType, MState, MPlayer, TestMResp, EndGameException

def ln():
  return inspect.currentframe().f_back.f_lineno

def checkresp(responses):
  def check(typ, d):
      r = responses.pop(0)
      try:
        if not (r[0] == typ and all([r[1][k] == d[k] for k in r[1]])):
          print("Got: " + str(typ))
          print(d)
          print("Expected: " + str(r[0]))
          print(r[1])
          print(r[2])
          return False
      except Exception:
        print("Got: " + str(typ))
        print(d)
        print("Expected: " + str(r[0]))
        print(r[1])
        print(r[2])
        return False
      if False:
        print("Correct: " + str(r[0]))
        print(r[1])
        print(r[2])
      return True
  return check

def genRoleGen(roles):
  def rolegen(playerids):
    players = {}
    for playerid,role in zip(playerids,roles):
      players[playerid] = MPlayer(playerid, role)
    return players
  return rolegen

class TestMState(unittest.TestCase):

  def test_easy(self):
    
    responses = [
      (MRespType.START, {}, ln()),
      (MRespType.VOTE_PLAYER, {'voter':'0','votee':'2','remain':1}, ln()),
      (MRespType.VOTE_PLAYER, {'voter':'1','votee':'2','remain':0}, ln()),
      (MRespType.ELECT, {'actor':'1', 'target':'2'}, ln()),
      (MRespType.TOWN_WIN, {}, ln()),
    ]

    roles = ['TOWN', 'TOWN', 'MAFIA']

    m = MState.fromPlayers(['0','1','2'], genRoleGen(roles), TestMResp(checkresp(responses)))

    self.assertEqual(m.phase, 'Day')
    self.assertEqual(m.day, 1)

    self.assertEqual(m.players['0'].vote, None)
    self.assertEqual(m.players['0'].role, "TOWN")
    self.assertEqual(m.players['0'].id, "0")

    m.vote('0','2')
    self.assertEqual(m.players['0'].vote, '2')
    try:
      m.vote('1','2')
      self.assert_(False, "Should never get here")
    except EndGameException:
      pass

  def test2(self):

    roles = ['TOWN', 'COP', 'DOCTOR', 'MAFIA']
    p_ids = ['0',    '1',   '2',      '3']
    responses = [
      (MRespType.START, {}, ln()),
      (MRespType.VOTE_PLAYER, {'voter':'3', 'votee':'2', 'remain':2}, ln()),
      (MRespType.VOTE_NOKILL, {'voter':'0', 'remain':1}, ln()),
      (MRespType.VOTE_NOKILL, {'voter':'1', 'remain':0}, ln()),
      (MRespType.ELECT, {'target':'NOTARGET'}, ln()),
      (MRespType.NIGHT, {}, ln()),

      (MRespType.TARGET, {'actor':'1', 'target':'3'}, ln()),
      (MRespType.TARGET, {'actor':'2', 'target':'2'}, ln()),
      (MRespType.MTARGET, {'actor':'3', 'target':'2'}, ln()),
      (MRespType.SAVE, {'actor':'2', 'target':'2', 'blocked':False, 'useful':True}, ln()),
      (MRespType.KILL, {'target':'2', 'success':False}, ln()),
      (MRespType.INVESTIGATE, {'actor':'1', 'target':'3'}, ln()),
      (MRespType.DAY, {}, ln()),

      (MRespType.VOTE_NOKILL, {'voter':'0', 'remain':1}, ln()),
      (MRespType.VOTE_NOKILL, {'voter':'1', 'remain':0}, ln()),
      (MRespType.ELECT, {'target':'NOTARGET'}, ln()),
      (MRespType.NIGHT, {}, ln()),

      (MRespType.TARGET, {'actor':'1', 'target':'2'}, ln()),
      (MRespType.TARGET, {'actor':'2', 'target':'NOTARGET'}, ln()),
      (MRespType.MTARGET, {'actor':'3', 'target':'2'}, ln()),
      (MRespType.KILL, {'target':'2', 'success':True}, ln()),
      (MRespType.INVESTIGATE, {'actor':'1', 'target':'2', 'sniped':True}, ln()),
      (MRespType.DAY, {}, ln()),

      (MRespType.VOTE_PLAYER, {'voter':'1', 'votee':'0', 'remain':1}, ln()),
      (MRespType.VOTE_PLAYER, {'voter':'3', 'votee':'0', 'remain':0}, ln()),
      (MRespType.ELECT, {'target':'0'}, ln()),
      (MRespType.MAFIA_WIN, {}, ln()),
    ]

    m = MState.fromPlayers(p_ids, genRoleGen(roles), TestMResp(checkresp(responses)))

    m.vote('3', '2')
    m.vote('0','NOTARGET')
    m.vote('1','NOTARGET')

    self.assertEqual(m.phase, 'Night')
    self.assertEqual(m.day, 1)

    m.target('1', '3')
    m.target('2','2')

    self.assertEqual(m.phase, 'Night')
    self.assertEqual(m.day, 1)

    m.mtarget('3','2')

    self.assertEqual(m.phase, 'Day')
    self.assertEqual(m.day, 2)

    m.vote('0','NOTARGET')
    m.vote('1','NOTARGET')

    self.assertEqual(m.phase, 'Night')
    self.assertEqual(m.day, 2)

    m.target('1', '2')
    m.target('2','NOTARGET')
    m.mtarget('3','2')

    self.assertEqual(m.phase, 'Day')
    self.assertEqual(m.day, 3)
    self.assertEqual(len(m.players), 3)

    m.vote('1','0')
    try:
      m.vote('3','0')
      self.assert_(False, "Should never get here")
    except EndGameException:
      pass

if __name__ == '__main__':
  unittest.main()
