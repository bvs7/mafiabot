import unittest
import sys
import inspect
import time
from contextlib import redirect_stdout

sys.path.append("C:\\Users\\Omniscient\\Documents\\Personal\\MafiaBot")

from mafiabot import MTimer

class test_MTimer(unittest.TestCase):

  def test_MTimer(self):

    with open("./test_MTimer_results.out",'w') as f:
      with redirect_stdout(f):

        def alarm0(n):
          print("Alarm 0: {}".format(n))

        def alarm1(n):
          print("Alarm 1: {}".format(n))

        def reminder0(n):
          print("Reminder 0: {}".format(n))

        def reminder1(n):
          print("Reminder 1: {}".format(n))

        m = MTimer(.6, [(alarm0, 0), (alarm1, .4)], [(reminder0, .2), (reminder1, .5)])

        time.sleep(.3)
        m.addTime(.3)
        print(m.getTime())
        time.sleep(.3)
        m.addTime(.3)
        print(m.getTime())
        m.addTime(-.5)
        print(m.getTime())

        m.cancel()

        time.sleep(1)


if __name__ == "__main__":
  unittest.main()