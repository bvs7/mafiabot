
import enum

from .MafiaState import PlayerID, ChatHandle, Phase, Role

import logging
logging.basicConfig(level=logging.DEBUG)

__all__ =["Event"]

class Event():
        
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

    def event(etype: EventType, phase_id: int, actor: PlayerID = None, target: PlayerID = None):
        logging.info(f"Logging {etype.name}, phase_id: {phase_id}, actor: {actor}, target: {target}")

    def addPlayerState(phase_id:int, player:PlayerID, role:Role):
        logging.info(f"Add PlayerState for phase_id {phase_id}: {player},{role}")

    def getPhaseID(lobby:ChatHandle, game_id:int, day:int, phase:Phase):
        """Retrieve phase id if it exists, or create it if it doesn't """
        logging.info(f"Getting Phase ID for {lobby, game_id, day, phase}")
        return 1
        
    def phaseState(lobby:ChatHandle, game_id:int, day:int, phase:Phase, players):
        phase_id = Event.getPhaseID(lobby, game_id, day, phase)
        for player in players:
            Event.addPlayerState(phase_id, player.id, player.role)

    def vote(phase_id : int, voter:PlayerID, votee:PlayerID):
        return Event.Event.event(Event.EventType.VOTE, phase_id, voter, votee)

    def target(phase_id : int, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.TARGET, phase_id, actor, target)

    def mtarget(phase_id : int, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.MAFIA_TARGET, phase_id, actor, target)

    def reveal(phase_id : int, celeb:PlayerID):
        return Event.event(Event.EventType.REVEAL, phase_id, celeb)

    def elect(phase_id:int, hammer:PlayerID, electee:PlayerID):
        return Event.event(Event.EventType.ELECT, phase_id, hammer, electee)

    def stun(phase_id:int, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.STUN, phase_id, actor, target)

    def save(phase_id:int, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.SAVE, phase_id, actor, target)
        
    def kill(phase_id:int, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.KILL, phase_id, actor, target)
        
    def investigate(phase_id:int, actor:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.INVESTIGATE, phase_id, actor, target)
        
    def revenge(phase_id:int, idiot:PlayerID, target:PlayerID):
        return Event.event(Event.EventType.REVENGE, phase_id, idiot, target)

    def eliminate(phase_id:int, target:PlayerID):
        return Event.event(Event.EventType.ELIMINATE, phase_id, None, target)

    def chargedie(phase_id:int, contracting:PlayerID, charge:PlayerID):
        return Event.event(Event.EventType.CHARGEDIE, phase_id, contracting, charge)

    def refocus(phase_id:int, contracting:PlayerID, new_charge:PlayerID):
        return Event.event(Event.EventType.REFOCUS, phase_id, contracting, new_charge)

    def start():
        # TODO: specific game data, rules, roles, players, etc?
        pass

    def win():
        pass
        # TODO: specific for game winning?
