from mafiabot import MState, MRules, MSaveEncoder, mafia_hook
import json


mstate = MState(1,MRules())
mstate.start(['1','2','3'], ['TOWN','TOWN','MAFIA'],{})
print(repr(mstate))
f = open("testout.maf",'w')

json.dump(mstate, f, cls= MSaveEncoder, indent=2)

f.close()

f = open("testout.maf",'r')
m = json.load(f, object_hook= mafia_hook)

print(repr(m))

f.close()

f = open("testout2.maf",'w')

json.dump(m, f, cls= MSaveEncoder, indent=2)

f.close()
