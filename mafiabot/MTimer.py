import time
import threading

class MTimer:

  def __init__(self, value, alarm, reminder=None):
    self.active = True

    self.end_time = time.time() + value
    
    self.cv = threading.Condition()
    full_time_thread = threading.Thread(name="Timer"+str(id), target=self.wait, args=(alarm,))
    full_time_thread.start()

    r = 5
    while value - r*60 > 0:
      reminder_thread = threading.Thread(target=self.wait_reminder, args=(reminder,))
      reminder_thread.start()
      r += 5

  def getRemindTime(self):
    toWait = self.end_time - time.time()
    minWait = toWait // 60
    nextMin = minWait - (minWait % 5)
    nextReminder = toWait - (nextMin * 60)
    return (nextReminder, nextMin)

  def wait_reminder(self, reminder):
    # wait until next 5 min increment
    with self.cv:
      (nextReminder, nextMin) = self.getRemindTime()
      while self.active:
        timeout = self.cv.wait(nextReminder)
        if timeout:
          break
        else:
          (nextReminder, nextMin) = self.getRemindTime()
      if self.active and reminder != None: # If notified and unnecessary, don't trigger!
        reminder(nextMin)
    return

  def wait(self, alarm):
    with self.cv:
      toWait = self.end_time - time.time()
      while self.active and toWait > 0:
        self.cv.wait(toWait)
        toWait = self.end_time - time.time()
      if self.active and alarm != None: # If notified and unnecessary, don't trigger!
        alarm()
    return

  def addTime(self, value):
    with self.cv:
      self.end_time += value
      self.cv.notify_all()
      # add additional reminders?
      # The ones that need to be added are those within the added window


  def getTime(self):
    with self.cv:
      t = self.end_time - time.time()
    return t

  def cancel(self):
    with self.cv:
      self.active = False
      self.cv.notify()
    
    