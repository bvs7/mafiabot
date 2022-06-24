
import enum

from .MafiaState import *

import logging
logging.basicConfig(level=logging.DEBUG)

__all__ =["Event", "EventHandler", "PrintEventHandler", "QueueTestEventHandler"]

class Event(enum.Enum):

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

    def handle(s, event:Event, *args, **kwargs):
        if hasattr(s, event.name):
            fn = getattr(s, event.name)
            del kwargs["self"]
            fn(*args, **kwargs)
        else:
            raise NotImplementedError

class MultiHandler(EventHandler):
    def __init__(s, handlers):
        s.handlers=handlers

    def handle(s, event:Event, *args, **kwargs):
        for handler in s.handlers:
            handler.handle(event, *args, **kwargs)

class PrintEventHandler:

    def handle(s, event, *args, **kwargs):
        logging.debug(f"Handle {event}:\n{args}{kwargs}")

class QueueTestEventHandler:

    def __init__(s, handler_on_empty=EventHandler()):
        s.event_queue = []
        s.handler_on_empty : EventHandler = handler_on_empty

    def check_empty(s):
        if len(s.event_queue) > 0:
            raise ValueError(s.event_queue)

    def queue(s, event, kwarg_dict=None):
        if kwarg_dict == None:
            kwarg_dict = {}
        s.event_queue.append((event, kwarg_dict))

    def handle(s, event, *args, **kwargs):
        if len(s.event_queue) == 0:
            return s.handler_on_empty.handle(event, *args, **kwargs)
        e, kwarg_dict = s.event_queue.pop(0)
        if not e == event:
            raise ValueError(event, e)
        for key in kwarg_dict:
            if not kwarg_dict[key] == kwargs[key]:
                raise ValueError(kwarg_dict[key],kwargs[key])


# class Event():

#     def event(etype: EventType, m, actor: PlayerID = None, target: PlayerID = None):
#         phase_id = Event.getPhaseIDFromGameState(m)
#         logging.info(f"Logging {etype.name}, phase_id: {phase_id}, actor: {actor}, target: {target}")

#     def addPlayerState(phase_id:int, player:PlayerID, role:Role):
#         logging.info(f"Add PlayerState for phase_id {phase_id}: {player},{role}")

#     def getPhaseID(lobby:ChatHandle, game_id:int, day:int, phase:Phase):
#         """Retrieve phase id if it exists, or create it if it doesn't """
#         logging.info(f"Getting Phase ID for {lobby, game_id, day, phase}")
#         return 1
    
#     def getPhaseIDFromGameState(m:GameState):
#         return Event.getPhaseID(m.lobby_chat, m.game_number, m.round.day,)
        
#     def phaseState(lobby:ChatHandle, game_id:int, day:int, phase:Phase, players):
#         phase_id = Event.getPhaseID(lobby, game_id, day, phase)
#         for player in players:
#             Event.addPlayerState(phase_id, player.id, player.role)

#     def vote(m:GameState, voter:PlayerID, votee:PlayerID):
#         return Event.event(Event.EventType.VOTE, m, voter, votee)

#     def target(m : GameState, actor:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.TARGET, m, actor, target)

#     def mtarget(m : GameState, actor:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.MAFIA_TARGET, m, actor, target)

#     def reveal(m : GameState, celeb:PlayerID):
#         return Event.event(Event.EventType.REVEAL, m, celeb)

#     def elect(m : GameState, hammer:PlayerID, electee:PlayerID):
#         return Event.event(Event.EventType.ELECT, m, hammer, electee)

#     def stun(m : GameState, actor:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.STUN, m, actor, target)

#     def save(m : GameState, actor:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.SAVE, m, actor, target)
        
#     def kill(m : GameState, actor:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.KILL, m, actor, target)
        
#     def investigate(m : GameState, actor:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.INVESTIGATE, m, actor, target)
        
#     def revenge(m : GameState, idiot:PlayerID, target:PlayerID):
#         return Event.event(Event.EventType.REVENGE, m, idiot, target)

#     def eliminate(m : GameState, target:PlayerID):
#         return Event.event(Event.EventType.ELIMINATE, m, None, target)

#     def chargedie(m : GameState, contracting:PlayerID, charge:PlayerID):
#         return Event.event(Event.EventType.CHARGEDIE, m, contracting, charge)

#     def refocus(m : GameState, contracting:PlayerID, new_charge:PlayerID):
#         return Event.event(Event.EventType.REFOCUS, m, contracting, new_charge)

#     def start():
#         # TODO: specific game data, rules, roles, players, etc?
#         pass

#     def win():
#         pass
#         # TODO: specific for game winning?
