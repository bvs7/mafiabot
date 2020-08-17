import time
import threading

class MTimer:
  """Used to create a timer object and bind functions to specific times for it"""
  
  def __init__(self, value, alarms, low_set_lim=0):
    """ value is the starting time value in seconds
    alarms is a dict mapping a second value to a list of alarm functions
    id is to help with debugging """
    self.value = max(value, low_set_lim)
    self.alarms = alarms
    self.active = True
    self.low_set_lim = low_set_lim
    
    self.lock = threading.Lock()

    self.tick_time = 1
    
    self.timerThread = threading.Thread(name="Timer"+str(id), target=self.tick)
    self.timerThread.start()
    print("Started")
    
  def tick(self):
    """ Internal function to count time """
    last_time = time.perf_counter()
    offset = 0
    while self.active:
      self.lock.acquire()
      for value,actions in self.alarms.items():
        if self.value == value:
          self.lock.release()
          for action in actions:
            action()
          self.lock.acquire()
      if self.value == 0:
        self.lock.release()
        self.active = False
        return
      self.value -= 1

      current_time = time.perf_counter()
      offset = (self.tick_time + offset) - (current_time - last_time)
      last_time = current_time
      self.lock.release()
      print("Tick: {}".format(offset))
      time.sleep(max([self.tick_time + offset, 0]))
      
  def addAlarms(self, alarms):
    with self.lock:
      for value,actions in alarms:
        if not value in self.alarms:
          self.alarms[value] = []
        for action in actions:
          self.alarms[value].append(action)
      
  def getTime(self):
    with self.lock:
      if not self.active:
        return None
      return self.value
      
  def addTime(self, seconds):
    with self.lock:
      if not self.active:
        return None
      self.value += seconds
      if self.value < self.low_set_lim:
        self.value = self.low_set_lim
        
  def halt(self):
    with self.lock:
      self.active = False
      self.alarms = {}
      self.value = 0
      
  
  def __str__(self):
    return time.strftime("%H:%M:%S",time.gmtime(self.value))

class FastMTimer(MTimer):
  def __init__(self, value, alarms, low_set_lim=0):
    super().__init__(value,alarms,low_set_lim)
    self.tick_time = .01
