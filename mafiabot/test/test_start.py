from .. import MController, TestMServer, TestMChat, TestMDM, MRoleGen, FastMTimer, MGame

import time

class testRoleGen:

  @staticmethod
  def roleGen(ids):
    return (ids, ["TOWN","COP","DOCTOR","MAFIA","STRIPPER"], {})

  @staticmethod
  def addIdiot(d, id):
    d[id] = ("IDIOT", id, False)
    return d


def voteData(votee):
  return {'attachments':[{'type':'mentions','user_ids':[votee]}]}

ctrl = MController(TestMChat, TestMDM, TestMServer, testRoleGen.roleGen, FastMTimer)

ctrl.handle_chat("30021302", "1", "in", "/in 5",{}) # 1: /in
ctrl.handle_chat("30021302", "2", "in", "/in 4",{}) # 2: /in
ctrl.handle_chat("30021302", "3", "in", "/in 3",{}) # 3: /in
ctrl.handle_chat("30021302", "4", "in", "/in 2",{}) # 4: /in
ctrl.handle_chat("30021302", "5", "in", "/in 1",{}) # 5: /in

ctrl.handle_chat("30021302", "1", "start", "/start 1 3",{}) # Start

time.sleep(5)

ctrl.handle_chat("MAIN CHAT", '1', 'vote', "/vote 4", voteData('4'))
time.sleep(.5)
ctrl.handle_chat("MAIN CHAT", '2', 'vote', "/vote 4", voteData('4'))

time.sleep(.5)

time.sleep(.5)
ctrl.handle_chat("MAIN CHAT", '4', 'vote', "/vote me", {})

time.sleep(.5)
ctrl.handle_dm("5","target","/target C", {})

time.sleep(.5)
ctrl.handle_dm("2","target","/target D", {})

time.sleep(.5)

time.sleep(.5)
ctrl.handle_dm("3","target","/target C", {})

time.sleep(.5)

time.sleep(.5)
ctrl.handle_chat("MAFIA CHAT", '5', 'target', '/target C', {})
time.sleep(.5)
ctrl.handle_chat("MAIN CHAT", '1', 'vote', "/vote 5", voteData('5'))

time.sleep(.5)

time.sleep(.5)
ctrl.handle_chat("MAIN CHAT", '2', 'vote', "/vote 5", voteData('5'))