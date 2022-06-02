
import enum

from .MafiaState import *

import logging
logging.basicConfig(level=logging.DEBUG)

__all__ =["Event"]

class EventType(enum.Enum):

    VOTE = "VOTE"
    TARGET = "TARGET"
    MAFIA_TARGET = "MAFIA_TARGET"
    REVEAL = "REVEAL"
    TIMER = "TIMER"

    START = "START"
    DAY = "DAY"
    NIGHT = "NIGHT"
    DUSK = "DUSK"
    ELECT = "ELECT"
    STUN = "STUN"
    SAVE = "SAVE"
    KILL = "KILL"
    INVESTIGATE = "INVESTIGATE"
    REVENGE = "REVENGE"
    ELIMINATE = "ELIMINATE"
    WIN = "WIN"
    CHARGEDIE = "CHARGEDIE"
    REFOCUS = "REFOCUS"

class EventHandler:
    def __init__(self):
        


class Event():
        
    

    # Define functions for submitting events?
    # PhaseID
    # phase_id -> lobby_id, game_id, day, phase
    # PlayerStates
    # phase_id -> player_id, player_role, (charge)

    # Elections
    # phase_id -> electee_id, voter_id

    # For each, phase_id
    # Event.EventType     actor id    target id
    # VOTE          voter       votee
    # TARGET        actor       target
    # MAFIA_TARGET  actor       target
    # REVEAL        celeb       -
    # TIMER         -           -

    # START 
    # DAY   (Make Phase ID) (phase)
    # NIGHT (Make Phase ID) (phase)
    # DUSK  (Make Phase ID) (phase)
    # ELECT         hammer      electee
    # STUN          actor       target
    # SAVE          actor       target
    # KILL          actor       target
    # INVESTIGATE   actor       target
    # REVENGE       idiot       target
    # ELIMINATE     -           target
    # WIN           team/player
    # CHARGEDIE     contracting charge
    # REFOCUS       contracting new_charge

    # Databases eventually...

    def event(etype: EventType, m, actor: PlayerID = None, target: PlayerID = None):
        phase_id = Event.getPhaseIDFromGameState(m)
        logging.info(f"Logging {etype.name}, phase_id: {phase_id}, actor: {actor}, target: {target}")

    def addPlayerState(phase_id:int, player:PlayerID, role:Role):
        logging.info(f"Add PlayerState for phase_id {phase_id}: {player},{role}")

    def getPhaseID(lobby:ChatHandle, game_id:int, day:int, phase:Phase):
        """Retrieve phase id if it exists, or create it if it doesn't """
        logging.info(f"Getting Phase ID for {lobby, game_id, day, phase}")
        return 1
    
    def getPhaseIDFromGameState(m:GameState):
        return Event.getPhaseID(m.lobby_chat, m.game_number, m.round.day,)
        
    def phaseState(lobby:ChatHandle, game_id:int, day:int, phase:Phase, players):
        phase_id = Event.getPhaseID(lobby, game_id, day, phase)
        for player in players:
            Event.addPlayerState(phase_id, player.id, player.role)

    def vote(m:GameState, voter:PlayerID, votee:PlayerID):
        return Event.event(Event.EventType.VOTE, m, voter, votee)

    def target(m : GameState, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.TARGET, m, actor, target)

    def mtarget(m : GameState, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.MAFIA_TARGET, m, actor, target)

    def reveal(m : GameState, celeb:PlayerID):
        return Event.event(Event.EventType.REVEAL, m, celeb)

    def elect(m : GameState, hammer:PlayerID, electee:PlayerID):
        return Event.event(Event.EventType.ELECT, m, hammer, electee)

    def stun(m : GameState, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.STUN, m, actor, target)

    def save(m : GameState, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.SAVE, m, actor, target)
        
    def kill(m : GameState, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.KILL, m, actor, target)
        
    def investigate(m : GameState, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.INVESTIGATE, m, actor, target)
        
    def revenge(m : GameState, idiot:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.REVENGE, m, idiot, target)

    def eliminate(m : GameState, target:PlayerID):
        return Event.event(Event.EventType.ELIMINATE, m, None, target)

    def chargedie(m : GameState, contracting:PlayerID, charge:PlayerID):
        return Event.event(Event.EventType.CHARGEDIE, m, contracting, charge)

    def refocus(m : GameState, contracting:PlayerID, new_charge:PlayerID):
        return Event.event(Event.EventType.REFOCUS, m, contracting, new_charge)

    def start():
        # TODO: specific game data, rules, roles, players, etc?
        pass

    def win():
        pass
        # TODO: specific for game winning?
