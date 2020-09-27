from mafiabot import MController, TestMServer, TestMChat, TestMDM, MRoleGen, FastMTimer

import time

class testRoleGen:

  @staticmethod
  def roleGen(ids):
    return (ids, ["TOWN","TOWN","TOWN","MAFIA","IDIOT"], testRoleGen.addIdiot({},ids[4]))

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

ctrl.handle_chat("MAIN CHAT", '1', 'vote', "/vote 5", voteData('5'))
time.sleep(.5)
ctrl.handle_chat("MAIN CHAT", '2', 'vote', "/vote 5", voteData('5'))
time.sleep(.5)
ctrl.handle_chat("MAIN CHAT", '5', 'vote', "/vote me", {})
