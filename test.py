from mafiabot import MState, MEventC

p = ['0','1','2','3','4']

ms = MState.from_players(p)

print(">Vote 0->1")
ms.vote('0','1')
print(">Vote 2->1")
ms.vote('2','1')
print(">Vote 3->1")
ms.vote('3','1')


print(">MTarget 2->0")
ms.mtarget('0')
print(">Target 3->2")
ms.target('3','2')
print(">Target 4->2")
ms.target('4','4')
