from json import *

from .MPlayer import MPlayer, MPlayerID, MRole
from .MState_refactor import MState, MVengeance, MContract, MPhase
from .MRules import MRules
from .MGame import MGame

__all__ = ['MSaveEncoder', 'mafia_hook']

class MSaveEncoder(JSONEncoder):
  def default(self, obj):
    if hasattr(obj, "to_json") and callable(obj.to_json):
      d = obj.to_json()
      name = "__{}__".format(obj.__class__.__name__)
      print(type(d))
      if isinstance(d, dict):
        d[name] = True
      else:
        d = {name:d}
      return d
    else:
      return JSONEncoder.default(self, obj)

hooks=[
  'MState',
  'MPlayer',
  'MVengeance',
  'MContract',
  'MPhase',
  'MGame',
  'MRules',
]

def mafia_hook(d):
  for hook in hooks:
    if "__{}__".format(hook) in d:
      try:
        result = globals()[hook].from_json(d)
      except Exception as e:
        print("MAFIA JSON ERROR reading {}: {}".format(str(d),e))
        raise NotImplementedError
      return result
  return d