
from typing import Union

""" MHandler is given a request, validates it, then performs any events it makes
"""
class MHandler:
  def __init__(self):
    self.request_queue = []
    # Create thread to run queue?

  def handle(self, request):
    # branch on every type of request this can be, perform resp/event
    pass