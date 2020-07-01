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
- num_voters
- num_f_voters
- num_players
- thresh
- no_kill_thresh

TARGET:
- actor
- target
- mafia

REVEAL:
- actor

TIMER:

START:

ELECT:
- actor
- target
- nokill

KILL:
- actor
- target
- success

VENGEANCE:
- actor
- target

ELIMINATE:
- actor
- target

CHARGE_DIE:
- actor
- target
- aggressor

DUSK:
- idiot
- actor
- target

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