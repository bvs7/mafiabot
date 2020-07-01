import time
import threading
from typing import List, Tuple, Callable

class MTimer:

  def __init__(self, value, alarms : List[Tuple[Callable[[],None],int]] = [], reminders : List[Tuple[Callable[[],None],int]] = [] ):
    self.active = True

    self.end_time = time.time() + value
    self.reminders = reminders
    
    self.cv = threading.Condition()
    for (alarm, t) in alarms:
      full_time_thread = threading.Thread(name="Timer"+str(id), target=self.wait, args=(alarm, t))
      full_time_thread.start()

    for (reminder, t) in reminders:
      reminder_thread = threading.Thread(target=self.wait_reminder, args=(reminder, t))
      reminder_thread.start()

  def createReminders(self, reminders):
    for (reminder, t) in reminders:
      if self.end_time - time.time() > t:
        reminder_thread = threading.Thread(target=self.wait_reminder, args=(reminder, t))
        reminder_thread.start()

  def wait_reminder(self, reminder, t):
    with self.cv:
      if self.active:
        toWait = self.end_time - time.time() - t
        timeout = not self.cv.wait(toWait)
        if timeout and self.active:
          reminder(t)

    return

  def wait(self, alarm, t):
    with self.cv:
      toWait = self.end_time - time.time() - t
      while self.active and toWait > 0:
        self.cv.wait(toWait)
        toWait = self.end_time - time.time() - t
      if self.active:
        alarm(t)
    return

  def addTime(self, value):
    with self.cv:
      self.end_time += value
      self.cv.notify_all()
      self.createReminders(self.reminders)
      time.sleep(.01)

  def getTime(self):
    with self.cv:
      t = self.end_time - time.time()
    return t

  def cancel(self):
    with self.cv:
      self.active = False
      self.cv.notify()
    
    