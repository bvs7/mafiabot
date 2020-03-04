from mafiabot import MState, MEventC

p =  [
    "0TOWN", "1TOWN", "2MAFIA",
    "3COP", "4DOCTOR", "5CELEB", "6MILLER",
    "7GODFATHER", "8STRIPPER", "9MILKY",
    "10IDIOT", "11SURVIVOR", "12TOWN", "13TOWN",
    "14GOON", "15MASON", "16MASON",
    "17GUARD", "18AGENT",
  ]

ms = MState.fromPlayers(p)

ms.vote("0TOWN", "10IDIOT")
ms.vote("1TOWN", "10IDIOT")
ms.vote("2MAFIA", "10IDIOT")
ms.vote("3COP", "10IDIOT")
ms.vote("4DOCTOR", "10IDIOT")
ms.vote('5CELEB', "10IDIOT")
ms.vote('6MILLER', "10IDIOT")
ms.vote('7GODFATHER', "10IDIOT")
ms.vote('8STRIPPER', "10IDIOT")
ms.vote('9MILKY', "10IDIOT")

# Night

ms.target("3COP","7GODFATHER")
ms.target("4DOCTOR", "0TOWN")
ms.mtarget("2MAFIA", "0TOWN")
ms.target("8STRIPPER", "4DOCTOR")
ms.target("9MILKY", "6MILLER")

# Day?

ms.vote("1TOWN", "2MAFIA")
ms.vote("2MAFIA", "2MAFIA")
ms.vote("3COP", "2MAFIA")
ms.vote("4DOCTOR", "2MAFIA")
ms.vote('5CELEB', "2MAFIA")
ms.vote('6MILLER', "2MAFIA")
ms.vote('7GODFATHER', "2MAFIA")
ms.vote('8STRIPPER', "2MAFIA")
ms.vote('9MILKY', "2MAFIA")

ms.timer()

ms.vote("1TOWN", "7GODFATHER")
ms.vote("17GUARD", "7GODFATHER")
ms.vote("3COP", "7GODFATHER")
ms.vote("4DOCTOR", "7GODFATHER")
ms.vote('5CELEB', "7GODFATHER")
ms.vote('6MILLER', "7GODFATHER")
ms.vote('7GODFATHER', "7GODFATHER")
ms.vote('8STRIPPER', "7GODFATHER")
ms.vote('9MILKY', "7GODFATHER")

ms.mtarget('8STRIPPER', '14GOON')
ms.target("3COP","NOTARGET")
ms.target("4DOCTOR", "NOTARGET")
ms.target("8STRIPPER", "NOTARGET")
ms.target("9MILKY", "9MILKY")

ms.vote("1TOWN", "8STRIPPER")
ms.vote("17GUARD", "8STRIPPER")
ms.vote("3COP", "8STRIPPER")
ms.vote("4DOCTOR", "8STRIPPER")
ms.vote('5CELEB', "8STRIPPER")
ms.vote('6MILLER', "8STRIPPER")
ms.vote('8STRIPPER', "8STRIPPER")

try:
  ms.vote('9MILKY', "8STRIPPER")
  raise AssertionError
except Exception as e:
  print(type(e))
