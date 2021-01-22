
from .MInfo import *
from .MPlayer import MPlayerID
from .MRole import MRole, TOWN_ROLES, MAFIA_ROLES, ROGUE_ROLES, CONTRACT_ROLES

import math
import random
from typing import Tuple, Set, Dict, NewType, Iterable, Union

class MContract:
  def __init__(self, role:MRole, charge:MPlayerID, success:bool):
    self.role=role
    self.charge=charge
    self.success=success

  def to_json(self):
    return {'role':self.role, 'charge':self.charge, 'success':self.success}

  @staticmethod
  def from_json(d):
    return MContract(d['role'],d['charge'],d['success'])

MAssignment = NewType('Assignment', Tuple[MPlayerID,Union[MRole,str]])
MRoleGenType = NewType('RoleGenType', Tuple[Iterable[MAssignment],Dict[MPlayerID,MContract]])

class MRoleGen:

  @staticmethod
  def roleGen(ids:MPlayerID) -> MRoleGenType:
    return MRoleGen.randomRoleGen(ids)

  @staticmethod
  def debugRoleGen(ids):
    n = len(ids)
    defaultRoles = ['TOWN','TOWN','MAFIA','COP','DOCTOR','IDIOT']
    contracts = {}
    if n >= 6:
      contracts[ids[5]] = ("IDIOT",ids[5],False)
    return defaultRoles[0:n], contracts

  @staticmethod
  def get_validity(n,n_maf,n_rogue):
    n_town = n-n_maf-n_rogue
    if n_maf < 1:
      return False
    if not n_town > n_rogue:
      return False
    return n_town >= n_maf+2

  @staticmethod
  def getNGauss(mu,sig,minimum,maximum=None):
    tries = 0
    n_role = math.floor(random.gauss(mu,sig))
    while not (n_role >= minimum and (maximum == None or n_role <= maximum)):
      tries += 1
      if tries >= 1000:
        return 0
      n_role = math.floor(random.gauss(mu,sig))
    return n_role

  @staticmethod
  def gaussNMafGen(n, mu_div = 5, u = .6, s = .15):
    mu = n/mu_div + u
    sig = s * math.sqrt(n)
    return MRoleGen.getNGauss(mu, sig, 1)


  @staticmethod
  def gaussNRogueGen(n, u=.4,s_div=5):
    mu = u
    sig = n/s_div
    return MRoleGen.getNGauss(mu,sig, 0)

  @staticmethod
  def gaussNTeamGen(n):
    # Gen n_rogue first then get num maf from that!
    n_rogue = MRoleGen.gaussNRogueGen(n)
    n_maf = MRoleGen.gaussNMafGen(n-n_rogue)
    while not MRoleGen.get_validity(n,n_maf,n_rogue):
      n_rogue = MRoleGen.gaussNRogueGen(n)
      n_maf = MRoleGen.gaussNMafGen(n-n_rogue)
    n_town = n-n_maf-n_rogue
    return n_town, n_maf, n_rogue

  # role_info is dict of role -> (score,dr)
  @staticmethod
  def selectRole(role_info):
    total = 0
    roles = list(role_info.keys())
    for role in roles:
      total += role_info[role]
    n = random.randrange(0,total)
    tot = 0
    for role in roles:
      tot += role_info[role]
      if n < tot:
        return role

    raise NotImplementedError("Shouldn't happen")


  @staticmethod
  def randomNumGen(num):
    n_town,n_maf,n_rogue = MRoleGen.gaussNTeamGen(num)

    n = n_town + n_maf + n_rogue

    roles = []

    # Decide n of roles of each in order?
    # STRIPPER, GODFATHER, MILLER, GOON => maf score?
    # maf score => COP, DOCTOR, MILKY, CELEB => maf score
    # maf score => ROGUES

    n_stripper = MRoleGen.getNGauss(0, math.sqrt(n_maf)*.7,0,n_maf)
    n_r_maf = n_maf - n_stripper
    n_godfather = MRoleGen.getNGauss(0, math.sqrt(n_r_maf)*.7,0,n_r_maf)
    n_r_maf = n_maf - n_stripper - n_godfather
    n_goon = MRoleGen.getNGauss(0, math.sqrt(n_maf)*.8,0,n_r_maf)
    n_mafia = n_maf - n_goon - n_godfather - n_stripper

    maf_score = n_mafia + 2*n_stripper + 1.5*n_godfather + (-1 if n_goon == n_maf else .5*n_goon)

    print('1')
    print(maf_score)

    # Static town roles
    n_miller = MRoleGen.getNGauss(.4, .25*math.sqrt(n_town), 0, math.sqrt(n_town))
    maf_score = maf_score + n_miller*.5  
    n_celeb = MRoleGen.getNGauss(.4, .25*math.sqrt(n_town), 0, math.sqrt(n_town))
    maf_score = maf_score - n_celeb

    print('2')
    print(maf_score)

    #TODO: Order these randomly?

    # Dynamic town roles
    n_cop = MRoleGen.getNGauss(maf_score/4 + .3, math.sqrt(max(0,maf_score)) * .5, 0, math.sqrt(n_town))
    maf_score = maf_score - n_cop*1.5
    n_doc = MRoleGen.getNGauss(maf_score/4 + .4, math.sqrt(max(0,maf_score)) * .5, 0, math.sqrt(n_town))
    maf_score = maf_score - n_doc*1.5
    n_milky = MRoleGen.getNGauss(maf_score/3, .56, 0, math.sqrt(n_town))
    maf_score = maf_score - n_milky

    print('3')
    print(maf_score)

    for i in range(n_maf):
      if n_stripper > 0:
        roles.append("STRIPPER")
        n_stripper -= 1
        continue
      if n_godfather > 0:
        roles.append("GODFATHER")
        n_godfather -= 1
        continue
      if n_goon > 0:
        roles.append("GOON")
        n_goon -= 1
        continue
      roles.append("MAFIA")
    
    for i in range(n_town):
      if n_cop > 0:
        roles.append("COP")
        n_cop -= 1
        continue
      if n_doc >0:
        roles.append("DOCTOR")
        n_doc -= 1
        continue
      if n_celeb > 0:
        roles.append("CELEB")
        n_celeb -= 1
        continue
      if n_miller > 0:
        roles.append("MILLER")
        n_miller -=1
        continue
      if n_milky > 0:
        roles.append("MILKY")
        n_milky -= 1
        continue
      roles.append("TOWN")

    rogues = []

    # Rogue roles
    for r in range(n_rogue):
      # buckets for maf_score
      if maf_score >= 1:
        # More likely Survivor, Guard(Town), Agent(Maf)
        role, target_team, score = MRoleGen.getAntiMafRogue()
      elif maf_score > -1:
        # Random
        role, target_team, score = MRoleGen.getRandRogue()
      else:
        # More likely Idiot, Guard(Maf), Agent(Town)
        role, target_team, score = MRoleGen.getProMafRogue()
      rogues.append((role,target_team))
      maf_score += score
      roles.append(role)

    print('4')
    print(maf_score)

    return roles, rogues

  @staticmethod
  def getScore(role, target_team):
    if role == "IDIOT":
      score = .5
    elif role == "SURVIVOR":
      score = 0
    elif role in ["GUARD","AGENT"]:
      if target_team == "Town":
        score = -1
      elif target_team == "Mafia":
        score = 1
      else:
        score = 0
      if role == "AGENT":
        score = score * -1
    return score

  @staticmethod
  def getAntiMafRogue():
    print("Anti")
    role = MRoleGen.selectRole({"SURVIVOR":20, "GUARD":40, "AGENT":40})
    target_team = "Town" if role == "GUARD" else "Mafia"
    return role, target_team, MRoleGen.getScore(role,target_team)

  @staticmethod
  def getRandRogue():
    print("Rand")
    role = MRoleGen.selectRole({"IDIOT":40, "SURVIVOR":40, "GUARD":10, "AGENT":10})
    target_team = MRoleGen.selectRole({"Town":50,"Mafia":30,"Rogue":20})
    return role, target_team, MRoleGen.getScore(role, target_team)

  @staticmethod
  def getProMafRogue():
    print("Pro")
    role = MRoleGen.selectRole({"IDIOT":20, "GUARD":40, "AGENT":40})
    target_team = "Town" if role == "AGENT" else "Mafia"
    return role, target_team, MRoleGen.getScore(role,target_team)

  @staticmethod
  def getTargetFromTeam(ids, roles, target_team):
    combo = zip(ids,roles)
    if target_team == "Mafia":
      targets = MAFIA_ROLES
    elif target_team == "Town":
      targets = TOWN_ROLES
    else:
      targets = ROGUE_ROLES
    possible = [c for c,r in combo if r in targets]
    if len(possible) > 0:
      return random.choice(possible)
    else:
      return random.choice(ids)

  @staticmethod
  def decideContracts(roles, rogues, ids):
    contracts = {}
    for id, role in zip(ids,roles):
      if role in CONTRACT_ROLES:
        success = role in ('SURVIVOR','GUARD')
        if role in ('IDIOT','SURVIVOR'):
          target = id
        elif role in ('GUARD','AGENT'):
          rogue_role, target_team = [(r,t) for (r,t) in rogues if r==role][0]
          target = MRoleGen.getTargetFromTeam(ids,roles,target_team)
          rogues.remove((rogue_role,target_team))
        contracts[id] = (role, target, success)
    return roles, contracts

  @staticmethod
  def randomRoleGen(ids):
    roles,rogues = MRoleGen.randomNumGen(len(ids))

    random.shuffle(roles)

    print(roles, rogues)

    # Should shuffle roles into ids, then decide rogue contracts based on rogues info
    roles,contracts = MRoleGen.decideContracts(roles, rogues, ids)

    assignments = list(zip(ids,roles))

    return assignments, contracts


"""
Types of Gaussian n role gen functions:
n_mafia := gauss(n/d+u,s*sqrt(n))
  mu scales with n
  sig widens as n gets larger
Creates a spike around n/d, wider as n grows

n_rogue := gauss(u, s*n)
  mu constant low (near zero)
  sig quickly grows with n
Creates a declining prob from 0 to ~1/3 n

n_stripper := gauss(n_maf/d+u, s*sqrt(n))
  mu scales with n_maf
  sig widens as n_maf gets larger
Creates a spike around n/d, wider as n grows

n_cop := gauss(n_maf/d + u, s)
  mu scales with n_maf
  sig constant, relatively wide
Creates a wide spike at n_maf/d (fair?)

n_doc := gauss(score/d + u, s)
  mu scales with score
  sig constant, wide

n_celeb := gauss(score/d + u, s)
  mu scales with score
  sig constant, wide

n_milky := gauss(u, s*n)
  mu constant low
  sig scales with n
Creates a declining prob from 0 to ~1/5 n_town?
"""