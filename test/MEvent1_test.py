import mafiabot
import time
import unittest

output_folder = "test_out/"

class BasicTests(unittest.TestCase):

  @staticmethod
  def evenOddTest(ut,f,start_night,even,should_be_night):
    def cast_main(msg):
        print("CAST MAIN: {}".format(msg),file=f)

    def cast_mafia(msg):
      print("CAST MAFIA: {}".format(msg),file=f)

    def send_dm(msg, p_id):
      print("SEND {}: {}".format(p_id, msg),file=f)

    rules = mafiabot.MRules()
    rules["start_night"] = start_night
    print(rules, file=f)
    state = mafiabot.MState(cast_main,cast_mafia,send_dm,rules)

    if even:
      ids = ['1','2','3','4']
      roles = ['TOWN','TOWN','MAFIA','TOWN']
    else:
      ids = ['1','2','3']
      roles = ['TOWN','TOWN','MAFIA']

    state.start(ids,roles)
    time.sleep(.1)
    if not should_be_night:
      ut.assertEqual(state.phase, mafiabot.MPhase.DAY)
      state.vote('1','NOTARGET')
      state.vote('2','NOTARGET')
    
    time.sleep(.1)
    ut.assertEqual(state.phase, mafiabot.MPhase.NIGHT)

    if even:
      state.mtarget('3','4')
    else:
      state.mtarget('3','NOTARGET')

    state.vote('1','3')
    state.vote('2','3')

    time.sleep(.1)
    ut.assertEqual(state.active, False)

    state.close()

  def test_basic1(self):

    with open(output_folder+"basic1.out", "w") as f:
      def cast_main(msg):
        print("CAST MAIN: {}".format(msg),file=f)

      def cast_mafia(msg):
        print("CAST MAFIA: {}".format(msg),file=f)

      def send_dm(msg, p_id):
        print("SEND {}: {}".format(p_id, msg),file=f)
    
      rules = mafiabot.MRules()
      state = mafiabot.MState(cast_main,cast_mafia,send_dm,rules)

      ids = ['1','2','3']
      roles = ['TOWN','TOWN','MAFIA']

      state.start(ids,roles)

      state.vote('1','3')
      state.vote('2','3')

      state.close()

  def test_start_night_basic(self):
    with open(output_folder+"start_night_basic1.out", "w") as f:
      self.evenOddTest(self, f, "ON", True, True)

    with open(output_folder+"start_night_basic2.out", "w") as f:
      self.evenOddTest(self, f, "ON", False, True)

    with open(output_folder+"start_night_basic3.out", "w") as f:
      self.evenOddTest(self, f, "EVEN", True, True)

    with open(output_folder+"start_night_basic4.out", "w") as f:
      self.evenOddTest(self, f, "EVEN", False, False)

    with open(output_folder+"start_night_basic5.out", "w") as f:
      self.evenOddTest(self, f, "ODD", True, False)

    with open(output_folder+"start_night_basic6.out", "w") as f:
      self.evenOddTest(self, f, "ODD", False, True)

    with open(output_folder+"start_night_basic7.out", "w") as f:
      self.evenOddTest(self, f, "OFF", True, False)

    with open(output_folder+"start_night_basic8.out", "w") as f:
      self.evenOddTest(self, f, "OFF", False, False)


  def test_fundamentals1(self):

    with open(output_folder+"fundamentals1.out", "w") as f:
      def cast_main(msg):
        print("CAST MAIN: {}".format(msg),file=f)

      def cast_mafia(msg):
        print("CAST MAFIA: {}".format(msg),file=f)

      def send_dm(msg, p_id):
        print("SEND {}: {}".format(p_id, msg),file=f)
    
      rules = mafiabot.MRules()
      state = mafiabot.MState(cast_main,cast_mafia,send_dm,rules)

      ids = ['1','2','3']
      roles = ['TOWN','TOWN','MAFIA']

      state.start(ids,roles)

      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 3)
      
      state.vote('1','2')

      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)

      state.vote('1',None)
      state.vote('2','2')

      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)

      state.vote('2','NOTARGET')
      state.vote('1','2')

      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertTrue(state.active)

      state.vote('3','2')

      time.sleep(.1)
      self.assertFalse(state.active)

      state.close()

  def test_fundamentals2(self):

    with open(output_folder+"fundamentals2.out", "w") as f:
      def cast_main(msg):
        print("CAST MAIN: {}".format(msg),file=f)

      def cast_mafia(msg):
        print("CAST MAFIA: {}".format(msg),file=f)

      def send_dm(msg, p_id):
        print("SEND {}: {}".format(p_id, msg),file=f)
    
      rules = mafiabot.MRules()
      state = mafiabot.MState(cast_main,cast_mafia,send_dm,rules)

      ids = ['1','2','3','4','5']
      roles = ['TOWN','TOWN','TOWN','MAFIA','TOWN']

      state.start(ids,roles)

      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 5)
      
      state.vote('1','2')

      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)

      state.vote('1','NOTARGET')
      state.vote('2','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)

      state.vote('3','5')
      state.vote('4','5')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)

      state.vote('5','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)

      state.mtarget('5','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 5)

      state.vote('3','NOTARGET')
      state.vote('4','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)

      state.vote('1','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)
      self.assertEqual(len(state.players), 5)

      state.mtarget('4','5')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 4)

      state.vote('1','2')
      state.vote('3','2')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 4)

      state.vote('1','NOTARGET')
      state.vote('3',None)
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 4)

      state.vote('1',None)
      state.vote('2','2')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 4)

      state.vote('4','2')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 4)

      state.vote('3','NOTARGET')
      state.vote('2','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)
      self.assertEqual(len(state.players), 4)

      state.mtarget('4','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 4)
      self.assertTrue(state.active)

      state.vote('1','4')
      state.vote('3','4')
      state.vote('2','4')
      time.sleep(.1)
      self.assertFalse(state.active)

      state.close()
  
  def test_doctor1(self):

    with open(output_folder+"doctor1.out", "w") as f:
      def cast_main(msg):
        print("CAST MAIN: {}".format(msg),file=f)

      def cast_mafia(msg):
        print("CAST MAFIA: {}".format(msg),file=f)

      def send_dm(msg, p_id):
        print("SEND {}: {}".format(p_id, msg),file=f)
    
      rules = mafiabot.MRules()
      rules["start_night"] = "ON"
      state = mafiabot.MState(cast_main,cast_mafia,send_dm,rules)

      ids = ['1','2','3']
      roles = ['TOWN','DOCTOR','STRIPPER']

      state.start(ids,roles)

      state.mtarget('3','2')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)
      self.assertEqual(len(state.players), 3)

      state.target('3','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)
      self.assertEqual(len(state.players), 3)

      state.target('2','2')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 3)

      state.timer()
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)
      self.assertEqual(len(state.players), 3)

      state.target('2','3')
      state.mtarget('3','1')
      state.target('2','1')
      state.target('3','NOTARGET')
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.DAY)
      self.assertEqual(len(state.players), 3)

      state.timer()
      time.sleep(.1)
      self.assertEqual(state.phase, mafiabot.MPhase.NIGHT)
      self.assertEqual(len(state.players), 3)

      state.mtarget('3','2')
      state.target('2','2')
      state.target('3','2')
      time.sleep(.1)
      self.assertFalse(state.active)

      state.close()