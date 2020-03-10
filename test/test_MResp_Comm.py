import unittest
import sys
import inspect
from contextlib import redirect_stdout

sys.path.append("C:\\Users\\Omniscient\\Documents\\Personal\\MafiaBot")
from mafiabot import MState, MResp_Comm, TestMComm, EndGameException, MRules, MRespType

class TestMResp_Comm(unittest.TestCase):

  def test_simple(self):

    output_fname = "test_simple.out"
    f = open(output_fname, 'w')
    with redirect_stdout(f):

      players = [n for n in range(0,5)]
      name_dict = {}
      for p in players:
        name_dict[str(p)] = chr(ord('A')+p)
      players = [str(p) for p in players]

      mrules = MRules()
      mrules['known_roles'] = "ROLE"
      mrules['reveal_on_death'] = "TEAM"
      mrules['know_if_saved_doc'] = "ON"
      mrules['know_if_saved_self'] = "ON"
      mrules['know_if_saved'] = "SAVED"
      mrules['cop_strength'] = "ROLE"

      mresp = MResp_Comm(
        TestMComm("MAIN",name_dict),
        TestMComm("MAFIA",name_dict),
        mrules,
      )

      m = MState.fromPlayers(
        players,
        mresp=mresp,
        mrules=mrules
      )

      m.timer()

      m.mtarget('2', 'NOTARGET')
      m.target('4','NOTARGET')
      m.target('3','NOTARGET')

      m.vote('0','1')
      m.vote('1','NOTARGET')

      m.mresp(MRespType.MAIN_STATUS, mstate=m)

      m.vote('1','0')
      m.vote('0','NOTARGET')

      m.mresp(MRespType.MAIN_STATUS, mstate=m)

      m.vote('1','2')

      m.vote('0',None)
      m.vote('1',None)

      m.mresp(MRespType.MAIN_STATUS, mstate=m)

      m.vote('0','NOTARGET')
      m.vote('1','NOTARGET')
      m.mresp(MRespType.MAIN_STATUS, mstate=m)
      m.vote('2', 'NOTARGET')

      # Night

      m.mtarget('2', '3')
      m.target('4','1')
      m.target('3','2')

      m.vote('0','2')
      m.vote('4','2')
      m.vote('1','NOTARGET')
      m.mresp(MRespType.MAIN_STATUS, mstate=m)

    f.close()
    # Test stdout

  def test_long(self):
    output_fname = "test_long.out"
    f = open(output_fname, 'w')
    with redirect_stdout(f):

      players = [n for n in range(0,20)]
      name_dict = {}
      for p in players:
        name_dict[str(p)] = chr(ord('A')+p)
      players = [str(p) for p in players]

      mrules = MRules()
      mrules['known_roles'] = "ROLE"
      mrules['reveal_on_death'] = "ROLE"
      mrules['know_if_saved_doc'] = "ON"
      mrules['know_if_saved_self'] = "OFF"
      mrules['know_if_saved'] = "SECRET"
      mrules['start_night'] = "EVEN"
      mrules['cop_strength'] = "ROLE"
      mrules['know_if_stripped'] = "USEFUL"

      mresp = MResp_Comm(
        TestMComm("MAIN",name_dict),
        TestMComm("MAFIA",name_dict),
        mrules,
      )

      m = MState.fromPlayers(
        players,
        mresp=mresp,
        mrules=mrules
      )

      m.target('3', '6')
      m.target('4', '6')
      m.target('8', '3')
      m.target('4', '18')
      m.target('9', '10')
      m.mtarget('2','18')
    f.close()

  def test_idiot(self):
    output_fname = "test_idiot.out"
    f = open(output_fname, 'w')
    with redirect_stdout(f):

      players = [n for n in range(0,11)]
      name_dict = {}
      for p in players:
        name_dict[str(p)] = chr(ord('A')+p)
      players = [str(p) for p in players]

      mrules = MRules()
      mrules['idiot_vengeance'] = "KILL"

      mresp = MResp_Comm(
        TestMComm("MAIN",name_dict),
        TestMComm("MAFIA",name_dict),
        mrules,
      )

      m = MState.fromPlayers(
        players,
        mresp=mresp,
        mrules=mrules
      )

      # m.vote('0','10')
      # m.vote('2','10')
      # m.vote('7','10')
      # m.vote('8','10')
      # m.vote('3','10')
      # m.vote('10','10')
      # m.mresp(MRespType.MAIN_STATUS, mstate=m)

      # # KILL
      # m.target('10','8')
      # m.mresp(MRespType.MAIN_STATUS, mstate=m)

      m.vote('0','2')
      m.vote('1','2')
      m.vote('2','2')
      m.vote('3','2')
      m.vote('4','2')
      m.vote('5','2')

      m.target('4','0')
      m.target('3','NOTARGET')
      m.mtarget('8','7')
      m.timer()

      m.vote('0','10')
      m.vote('1','10')
      m.vote('8','10')
      m.vote('3','10')
      m.vote('10','10')

      try:
        m.target('10','8')
        assert False, "Shouldn't get here"
      except EndGameException:
        pass


      

    f.close()

if __name__ == '__main__':
  unittest.main()