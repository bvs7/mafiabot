__Next States__

VOTE:
-  +ELECT

TARGET:
-  +DAWN|+VENGEANCE

REVEAL:

TIMER:
-  +DAWN|+NIGHT

START:
-  +DAY|+NIGHT

ELECT:
-  +ELIMINATE|+DUSK
-  +NIGHT|

KILL:
-  +ELIMINATE

VENGEANCE:
-  ++ELIMINATE
-  +NIGHT

ELIMINATE:
-  ++CHARGE_DIE
-  +WIN

CHARGE_DIE:
-  +REFOCUS

DUSK:

REFOCUS:

WIN:
-  ++CONTRACT_RESULT
-  +END

CONTRACT_RESULT:

END:

NIGHT:

DAWN:
-  ++STRIP
-  ++SAVE
-  +KILL
-  ++MILK
-  ++INVESTIGATE
-  +DAY

STRIP:

SAVE:

MILK:

INVESTIGATE:

DAY:

__Contents of Events__

VOTE:
- voter
- votee
- *f_votee
- *num_voters
- *num_f_voters
- *num_players
- *thresh
- *no_kill_thresh

TARGET:
- actor
- target
- mafia
- *players
- *phase

REVEAL:
- actor
- *stripped

TIMER:
- *phase

START:
- ids
- roles
- *phase

ELECT:
- actor
- target
- nokill
- *role (of target)

KILL:
- actor
- target
- success
- *role (of target)

VENGEANCE:
- actor
- target
- *role (of target)
- vengeance

ELIMINATE:
- actor
- target
- role (of target)

CHARGE_DIE:
- actor
- target
- aggressor

DUSK:
- idiot
- actor
- target
- venges

REFOCUS:
- actor
- target
- aggressor
- role

WIN:
- winning_team

CONTRACT_RESULT:
- contractor
- role
- charge
- success

END:
- winning_team

NIGHT:

DAWN:
- strips
- saves
- kill
- milks
- investigates

STRIP:
- actor
- target
- success

SAVE:
- actor
- target
- stripped
- success

MILK:
- actor
- target
- stripped
- success

INVESTIGATE:
- actor
- target
- stripped
- success

DAY:


__ORDERING__
START
TIMER
NIGHT
MTARGET
TARGET
DAWN
STRIP
SAVE
KILL
MILK
INVESTIGATE
DAY
REVEAL
VOTE
ELECT
DUSK
VENGEANCE
ELIMINATE
CHARGE_DIE
REFOCUS
WIN
CONTRACT_RESULT
END