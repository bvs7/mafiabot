from typing import Union
from MPlayer import MPlayer

class MTarget:
  def __init__(self, target: Union[MPlayer, None]):
    self.target = target # Either an MPlayer or None for nokill