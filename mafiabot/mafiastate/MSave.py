# Encoder for saving mafia states, and hooks for decoding them.

import json
from .. import mafiastate # pylint: disable

class MSaveEncoder(json.JSONEncoder):
  def default(self, obj):
    if hasattr(obj, "to_json") and callable(obj.to_json):
      d = obj.to_json()
      name = "__{}__".format(obj.__class__.__name__)
      if isinstance(d, dict):
        d[name] = True
      else:
        d = {name:d}
      return d
    else:
      return json.JSONEncoder.default(self, obj)

# For each of these hook classes,
#  They have a from_json(d) fn which takes a dictionary with d['__[hook]__'] is d
#  That fn sets its attributes based on entries in d[].

hooks=[
  'MState',
  'MPlayer',
  'MVengeance',
  'MContract',
  'MPhase',
  'MGame',
  'MRules',
  'MRole',
  'MTeam',
]

def mafia_hook(d):
  for hook in hooks:
    if "__{}__".format(hook) in d:
      try:
        result = getattr(mafiastate,hook).from_json(d)
      except Exception as e:
        print("MAFIA JSON ERROR reading {}: {}".format(str(d),str(e)))
        raise e
      return result
  return d

def msave(obj, f):
  json.dump(obj, f, cls=MSaveEncoder, indent=2)

def mload(f):
  return json.load(f, object_hook=mafia_hook)