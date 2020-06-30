import unittest
import sys
import inspect
from contextlib import redirect_stdout
import threading
import asyncio
import time

sys.path.append("C:\\Users\\Omniscient\\Documents\\Personal\\MafiaBot")
from mafiabot import MState, MResp_Comm, TestMComm, EndGameException, MRules, MRespType
from mafiabot import MDiscordComm, MDiscordClient, DISCORD_TOKEN

def run_client(client):
  client.run(DISCORD_TOKEN)

def run_coroutine(f, *args, **kwargs):
    loop = asyncio.get_event_loop()
    result = loop.run_until_complete(f(*args, **kwargs))
    loop.close()
    return result

class TestMResp_Comm(unittest.TestCase):

  def test_simple(self):
    try:
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
        mrules['start_night'] = "ON"

        client = MDiscordClient()
        cthread = threading.Thread(target=run_client, args=(client,))
        cthread.start()

        mguild = client.guilds[0]
        main_channel = [g for g in mguild.channels if g.name == "test_main_chat"][0]
        mafia_channel = [g for g in mguild.channels if g.name == "test_mafia_chat"][0]

        mresp = MResp_Comm(
          MDiscordComm(client, main_channel),
          MDiscordComm(client, mafia_channel),
          mrules,
        )

        m = MState.fromPlayers(
          players,
          mresp=mresp,
          mrules=mrules
        )

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
    except Exception as e:
      print(e)
    finally:
      f.close()
      # Test stdout
      run_coroutine(client.close)

if __name__ == '__main__':
  unittest.main()