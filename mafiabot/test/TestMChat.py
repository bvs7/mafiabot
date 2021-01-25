from mafiabot import MChat
import re
from collections import deque

class TestMChatError(Exception):
  pass

class TestMChat(MChat):
  """ TestMChat. When these are created, use the group_id, which is unique to
  the name when new() is called, to find a file with expected outputs?

  Use some simple regexp ideas to make the output files more advanved...
  Each line can be a regexp if it is in [].
  Just a * will accept everything until the desired line appears
  Don't do two * in a row!
  """

  def __init__(self, group_id, name_reference=None):
    try:
      fname = "chat_{}_expected.msg".format(group_id)
      f = open(fname)
    except OSError:
      super().__init__(group_id, name_reference)
      return
    
    self.state = "NORMAL"
    self.lines = self.parse(f.readlines())
    f.close()

  def parse(self, lines):
    results = deque()
    for line in lines:
      line = line.strip()
      if line[0] == "#" or len(line) == 0:
        continue
      if line[0] == "[" and line[-1] == "]":
        results.append(("regexp",line[1:-1]))
      elif line == "*":
        results.append(("star",line))
      elif line[0] == "{" and line[-1] == "}":
        results.append(("ignore_n",line))
      else:
        results.append(("standard",line))
    return results

  def cast(self,msg):
    # Compare msg to lines.
    msg = msg.replace("\n", ' ')
    self.check(msg)


  def check(self, msg):
    if isinstance(self.state, int):
      print("Matched: {}, {} \n {}".format("ignore_n", self.state, msg))
      self.state -= 1
      if self.state <= 0:
        self.state = "NORMAL"
      return True
    t,expected = self.lines.popleft()
    try:
      if t == "standard":
        if not msg == expected:
          raise TestMChatError(msg, t, expected)
        print("Matched: {}, {} \n {}".format(t, expected, msg))
        result = True
      elif t == "regexp":
        exp = re.compile(expected, re.IGNORECASE)
        if exp.fullmatch(msg) == None:
          raise TestMChatError(msg, t, expected)
        print("Matched: {}, {} \n {}".format(t, expected, msg))
        result = True
      elif t == "star":
        self.state = "STAR"
        print("State is now Star")
        result = self.check(msg)
      elif t == "ignore_n":
        self.state = int(expected)
        result = self.check(msg)
    except TestMChatError as e:
      if not self.state == "STAR":
        raise e
      self.lines.appendleft((t,expected))
      print("Matched: {}, {} (next: {}, {}) \n {}".format("star","*", t, expected, msg))
      result = True
    return result