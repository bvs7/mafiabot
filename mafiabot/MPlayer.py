# from MTarget import NullTarget, NoTarget, PlayerTarget

# from enum import Enum

class MPlayer:
  def __init__(self, id, role):
    self.id = id
    self.vote = None
    self.role = role
    self.target = None

# class MPlayer:
#   def __init__(self, id):
#     self.id = id;
#     self.vote = NullTarget()
    
#     if isinstance(self, TargetPlayer):
#       self.target = NullTarget
#     if isinstance(self, ChargePlayer):
#       self.charge = None # Must be set after initialization
    
#   def __str__(self):
#     return self.__name__
    
# class TargetPlayer:
#   pass
    
# class ChargePlayer:
#   pass
    
# class MPlayerTown(MPlayer):
#   pass
  
# class MPlayerMafia(MPlayer):
#   pass

# class MPlayerRogue(MPlayer):
#   pass
    
# class TOWN(MPlayerTown):
#   pass
  
# class COP(MPlayerTown, TargetPlayer):
#   pass
  
# class DOCTOR(MPlayerTown, TargetPlayer):
#   pass
  
# class CELEB(MPlayerTown):
#   def __init__(self, id):
#     super().__init__(id)
#     self.revealed = False
  
# class MILLER(MPlayerTown):
#   pass

# class MILKY(MPlayerTown, TargetPlayer):
#   pass
  
# class MASON(MPlayerTown):
#   pass
  
# class MAFIA(MPlayerMafia):
#   pass
 
# class GODFATHER(MPlayerMafia):
#   pass
  
# class STRIPPER(MPlayerMafia, TargetPlayer):
#   pass
  
# class GOON(MPlayerMafia):
#   pass
  
# class IDIOT(MPlayerRogue):
#   pass
  
# class GUARD(MPlayerRogue, ChargePlayer):
#   pass

# class AGENT(MPlayerRogue, ChargePlayer):
#   pass
