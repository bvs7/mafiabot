
from .MInfo import *

import math
import random

def roleGen(ids):
  return debugRoleGen(ids)

def debugRoleGen(ids):
  n = len(ids)
  defaultRoles = ['TOWN','TOWN','MAFIA','COP','DOCTOR','IDIOT']
  contracts = {}
  if n >= 6:
    contracts[ids[5]] = ("IDIOT",ids[5],False)
  return defaultRoles[0:n], contracts

def get_validity(n,n_maf,n_rogue):
  n_town = n-n_maf-n_rogue
  if n_maf < 1:
    return False
  if not n_town > n_rogue:
    return
  if n % 2 == 0:
    # Even, non maf must be at least 2 greater
    return n_town >= n_maf+2
  else: # Odd, at least 1 greater
    return n_town >= n_maf+1

def randomNMafGen(n, alpha=1.2, x=.1):
  n_maf = n
  n_rogue = 0

  while not get_validity(n,n_maf,n_rogue):
    n_maf = n/4
    r = random.gauss(0, .75) # r is maf advantage number
    n_maf = math.floor(n_maf + r)

  n_rogue = math.floor(random.gammavariate(alpha, n/n_maf*x))
  while not get_validity(n,n_maf,n_rogue):
    n_rogue = math.floor(random.gammavariate(alpha, n/n_maf*x))

  return n, n_maf, n_rogue, 2*r

def test_maf(n, alpha=1.2, x=.1):

  d = {}
  for i in range(1000):
    n,n_maf,n_rogue, _ = randomNMafGen(n,alpha,x)
    if not (n_maf,n_rogue) in d:
      d[(n_maf,n_rogue)] = 0
    d[(n_maf,n_rogue)] += 1
  return d

def show(d, s=5):           
  msg = ""
  for i in range(s):
    for j in range(-1, s):
      if j == -1:
        msg += "{:>3}".format(i)
      elif i == 0:
        msg += "{:>3}".format(j)
      elif (i,j) in d:
        msg += "{:>3}".format(d[(i,j)])
      else:
        msg += "___"
      msg += " "
    msg += "\n"
  print(msg)

default_town_roles = {
  'TOWN':(75,0),
  'COP':(10,-.6),
  'DOCTOR':(10,-.7),
  'CELEB':(10,-.6),
  'MILKY':(5,-.4),
  'MILLER':(10,.2),
}

default_mafia_roles = {
  'MAFIA':(75,0),
  'GODFATHER':(5,.2),
  'STRIPPER':(10,.7),
  'GOON':(10,-.5),
}

default_rogue_roles = {
  'IDIOT':(25,.5),
  'SURVIVOR':(25,.3),
  'GUARD':(20,.1),
  'AGENT':(15,.3),
}

# role_info is dict of role -> (score,dr)
def selectRole(role_info):
  total = 0
  roles = list(role_info.keys())
  for role in roles:
    total += role_info[role][0]
  n = random.randrange(0,total)
  tot = 0
  for role in roles:
    tot += role_info[role][0]
    if n < tot:
      return role, role_info[role][1]

  raise NotImplementedError("Shouldn't happen")

def randomNumGen(num, town_roles=default_town_roles,
    mafia_roles=default_mafia_roles, rogue_roles=default_rogue_roles):
  n,n_maf,n_rogue,r = randomNMafGen(num)

  n_town = n-n_maf-n_rogue

  roles = []

  for t in range(n_town):
    # randomly select a town role
    role, dr = selectRole(town_roles)
    roles.append(role)
    r += dr

  for m in range(n_maf):
    role, dr = selectRole(mafia_roles)
    roles.append(role)
    r += dr

  for i in range(n_rogue):
    role, dr = selectRole(rogue_roles)
    roles.append(role)
    r += dr

  return roles,r

def helpMaf(roles):
  new_roles = []
  changes = 0
  for role in roles:
    if not changes > 1:
      if role in ['MAFIA','GOON']:
        new_roles.append(random.choice(['GODFATHER','STRIPPER']))
        changes += 2
        continue
    new_roles.append(role)

  if changes > 1:
    return new_roles

  new_roles1 = []

  for role in new_roles:
    if not changes > 1:
      if role in ['COP','DOCTOR','CELEB']:
        new_roles1.append('TOWN')
        changes += 1
        continue
    new_roles1.append(role)

  if changes > 1:
    return new_roles1

  new_roles2 = []
  for role in new_roles1:
    if not changes > 1:
      if role == 'GOON':
        new_roles2.append('MAFIA')
        changes += 1
        continue
    new_roles2.append(role)

  if changes > 1:
    return new_roles2

  new_roles3 = []
  for role in new_roles2:
    if not changes > 1:
      if role in ('IDIOT', 'SURVIVOR', 'GUARD', 'AGENT'):
        new_roles3.append('MAFIA')
        changes += 1
        continue
    new_roles3.append(role)

  return new_roles3

def helpTown(roles):
  new_roles = []
  changes = 0

  for role in roles:
    if not changes > 1:
      if role in ['MILLER', 'TOWN']:
        new_roles.append(random.choice(['COP','DOCTOR','CELEB']))
        changes += 2
        continue
    new_roles.append(role)

  if changes > 1:
    return new_roles

  new_roles1 = []

  for role in new_roles:
    if not changes > 1:
      if role in ['GODFATHER', 'STRIPPER']:
        new_roles1.append('MAFIA')
        changes += 1
        continue
    new_roles1.append(role)

  if changes > 1:
    return new_roles

  new_roles2 = []
  for role in new_roles1:
    if not changes > 1:
      if role == 'MILLER':
        new_roles2.append('TOWN')
        changes += 1
        continue
    new_roles2.append(role)

  if changes > 1:
    return new_roles2

  new_roles3 = []
  for role in new_roles2:
    if not changes > 1:
      if role in ('IDIOT', 'SURVIVOR', 'GUARD', 'AGENT'):
        new_roles3.append('TOWN')
        changes += 1
        continue
    new_roles3.append(role)

  return new_roles3

def randomRoleGen(ids, town_roles=default_town_roles,
    mafia_roles=default_mafia_roles, rogue_roles=default_rogue_roles):
  roles,r = randomNumGen(len(ids), town_roles,mafia_roles,rogue_roles)

  while r < -1:
    roles = helpMaf(roles)
    r += 1

  while r > 1:
    roles = helpTown(roles)
    r -= 1

  random.shuffle(roles)

  contracts = {}

  for id,role in zip(ids,roles):
    if role in CONTRACT_ROLES:
      success = role in ('SURVIVOR','GUARD')
      if role in ('IDIOT','SURVIVOR'):
        target = id
      elif role in ('GUARD','AGENT'):
        target = random.choice([i for i in ids if i != id])
      contracts[id] = (role, target, success)
  
  return ids, roles, contracts
    