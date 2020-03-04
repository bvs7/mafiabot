from mafiabot import MState, MResp_GroupMe, TestMComm

m = MState.fromPlayers(['0','1','2'], mresp=MResp_GroupMe(TestMComm("MAIN",{'0':'Zero','1':'One','2':'Two'})))
