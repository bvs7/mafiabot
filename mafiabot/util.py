from enum import Enum, EnumMeta

class VEnumMeta(EnumMeta):
  def __call__(cls, value, names=None, *, module=None, qualname=None, type=None, start=None):
    vnames = None
    if names is None:
      # Implement VEnum("member") -> VEnum.member
      if isinstance(value, str):
        if value in cls.__dict__:
          return cls.__dict__[value]
      return super().__call__(value,names, module=module,qualname=qualname,type=type,start=start)
    if isinstance(names, str):
      vnames = names.replace(',', ' ').split()
      vnames = [(name,name) for name in vnames]
    if isinstance(names, (tuple,list)) and names and isinstance(names[0],str):
      #iterable of str
      vnames = [(name,name) for name in names]
    if not vnames: # wasn't one of the above types
      for item in names:
        if isinstance(item,str):
          vnames.append((item,item))
        else: # tuple list of name, value
          vnames.append((item[0],item[0]))
    return super().__call__(value,vnames,module=module,qualname=qualname,type=type,start=start)

  def __contains__(cls, item):
    item = cls.__init__(item)
    item.name in cls._member_map_

class VEnum(Enum, metaclass=VEnumMeta):
  def __repr__(self):
    return "<%s.%s>" % (self.__class__.__name__, self.name)
  
  def __eq__(self, other):
    if isinstance(other, str):
      return self.name == other
    if isinstance(other, self.__class__):
      return self.name == other.name
    return False

  def __hash__(self):
    return hash(self.name)

class OrderedVEnum(VEnum):
  def __lt__(self, other):
    other = self.__class__(other)
    l = list(self.__class__)
    return l.index(self) < l.index(other)

  def __le__(self,other):
    return self == other or self < other

# defined with dynamic getattr to prevent pylint errors
class auto:
  def __getattr__(self, name):
    return self.__getattribute__(name)