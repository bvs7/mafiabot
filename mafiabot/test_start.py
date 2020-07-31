from mafiabot import MController, TestMServer, TestMChat, TestMDM

import time

def debug(ctrl):
  ctrl.handle_chat("30021302", "1", "in", "",{}) # 1: /in
  ctrl.handle_chat("30021302", "2", "in", "",{}) # 2: /in
  ctrl.handle_chat("30021302", "3", "in", "",{}) # 3: /in
  ctrl.handle_chat("30021302", "4", "in", "",{}) # 4: /in
  ctrl.handle_chat("30021302", "5", "in", "",{}) # 5: /in
  ctrl.handle_chat("30021302", "6", "in", "",{}) # 6: /in

  ctrl.handle_chat("30021302", "1", "start", "",{}) # Start

  time.sleep(1)

  ctrl.handle_chat("MAFIA CHAT", '3', 'target', '/target A', {})
  ctrl.handle_dm("4", 'target', '/target A', {})
  ctrl.handle_dm("5", 'target', '/target A', {})


ctrl = MController(TestMChat, TestMDM, TestMServer, debug)
