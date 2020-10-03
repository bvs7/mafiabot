import mafiabot
import json

mstate = mafiabot.MState_refactor.MState(1,mafiabot.MRules())

f = open("testout.maf",'w')

json.dump(mstate, f, cls=mafiabot.MSave.MSaveEncoder, indent=2)

f.close()

f = open("testout.maf",'r')
m = json.load(f, object_hook=mafiabot.MSave.mafia_hook)

f.close()
