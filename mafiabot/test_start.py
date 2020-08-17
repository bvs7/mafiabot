from mafiabot import MController, TestMServer, TestMChat, TestMDM

import time

def debug(ctrl):
  ctrl.handle_chat("30021302", "1", "in", "/in 4",{}) # 1: /in
  ctrl.handle_chat("30021302", "2", "in", "/in 5",{}) # 2: /in
  ctrl.handle_chat("30021302", "3", "in", "/in 3",{}) # 3: /in
  ctrl.handle_chat("30021302", "4", "in", "/in 3",{}) # 4: /in
  ctrl.handle_chat("30021302", "5", "in", "/in 5",{}) # 5: /in

  ctrl.handle_chat("30021302", "1", "start", "/start 1 3",{}) # Start

  time.sleep(1)

#  ctrl.handle_chat("MAFIA CHAT", '3', 'target', '/target A', {})
 # ctrl.handle_dm("4", 'target', '/target A', {})
  #ctrl.handle_dm("5", 'target', '/target A', {})


ctrl = MController(TestMChat, TestMDM, TestMServer, debug)
