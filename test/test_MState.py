import unittest

import mafiabot
from mafiabot import MState

class TestMState(unittest.TestCase):

  def test_vote(self):
    m = MState.MState()
    self.assertTrue(callable(getattr(MState.handle_vote)))
