
from typing import List, Tuple

import logging
logging.basicConfig(level=logging.DEBUG)

from .MafiaUtil import *
from .MafiaRules import *
from .MafiaState import *
from .MafiaEvent import *

class MafiaController:

    def __init__(self, handler:EventHandler = EventHandler()):
        self.handle = handler.handle
    
    def start(self, chat:ChatHandle, game_number:int, role_list:List[Tuple[PlayerID,Role]], rules:RuleSet):

        players = set()
        for p_id,role in role_list:
            players.add(Player(p_id, role))
        return GameState(chat, game_number, players=players, rules=rules)

    def vote(self, m:GameState, voter:PlayerID, votee:PlayerID):
        if not m.round.phase == Phase.DAY:
            raise MafiaFormatError("Can only vote during Day")
        m.round.votes[voter] = votee
        thresh, count = self.check_vote_thresh(m, votee)
        self.handle(Event.VOTE, **locals())
        if count >= thresh:
            self.elect(m, voter, votee)

    def unvote(self, m:GameState, voter:PlayerID):
        if not m.round.phase == Phase.DAY:
            raise MafiaFormatError("Can only vote during Day")
        if not voter in m.round.votes:
            raise MafiaFormatError("Cannot unvote without a vote")
        f_votee = m.round.votes[voter]
        del m.round.votes[voter]
        votee = None
        thresh, count = self.check_vote_thresh(m, f_votee)
        self.handle(Event.VOTE, **locals())

    def target(self, m:GameState, actor:PlayerID, target:PlayerID):
        actor_p = m.getPlayer(actor)

        if m.round.phase == Phase.NIGHT:
            if not actor_p.role.targeting:
                raise MafiaFormatError("Targeting roles must target at NIGHT")
        elif m.round.phase == Phase.DUSK:
            if not m.round.idiot == actor:
                raise MafiaFormatError("Only the revenge-seeking idiot can target at DUSK")
        else:
            raise MafiaFormatError("Targeting cannot be done during the DAY")


        if m.round.phase == Phase.NIGHT:
            m.round.targets[actor] = target
            self.handle(Event.TARGET, **locals())
            self.check_to_day(m)
            return

        if m.round.phase == Phase.DUSK:
            if not target in m.round.voters:
                raise MafiaFormatError("Revenge-seeking idiot must select a voter")
            self.handle(Event.TARGET, **locals())
            self.revenge(m, actor, target)
            return

    def untarget(self, m:GameState, actor:PlayerID):
        actor_p = m.getPlayer(actor)

        if not actor_p.role.targeting:
            raise MafiaFormatError("Only targeting roles need to untarget")
        if not m.round.phase == Phase.NIGHT:
            raise MafiaFormatError("Can only untarget at NIGHT")
        if not actor in m.round.targets:
            raise MafiaFormatError("Cannot untarget without target")

        del m.round.targets[actor]
        target = None
        self.handle(Event.TARGET, **locals())

    def mafia_target(self, m:GameState, actor:PlayerID, target:PlayerID):
        actor_p = m.getPlayer(actor)
        if not m.round.phase == Phase.NIGHT:
            raise MafiaFormatError("Can only mafia target at NIGHT")
        if actor_p.role == Role.GOON and not target == PlayerID.NOTARGET:
            raise MafiaFormatError("GOON can only target NO TARGET")
        
        m.round.mafia_target = target
        self.handle(Event.MAFIA_TARGET, **locals())

        self.check_to_day(m)

    def mafia_untarget(self, m:GameState, actor:PlayerID):
        actor_p = m.getPlayer(actor)
        if not m.round.phase == Phase.NIGHT:
            raise MafiaFormatError("Can only mafia untarget at NIGHT")
        if m.round.mafia_target == None:
            raise MafiaFormatError("No target to untarget")
        m.round.mafia_target = None
        target = None
        self.handle(Event.MAFIA_TARGET, **locals())

    def revenge(self, m:GameState, idiot:PlayerID, target:PlayerID):
        logging.info(f"{idiot} gets revenge on {target}")
        self.eliminate(target)
        self.eliminate(idiot)
        self.night()

    def eliminate(self, m:GameState, p_id:PlayerID):
        p = m.getPlayer(p_id)
        # TODO: Check for victory
        logging.info(f"{p_id} died")

        m.players.remove(p)

        self.check_win(m)

    def check_win(self, m:GameState):
        n = len(m.players)
        n_town = 0
        n_maf = 0
        for p in m.players:
            if p.role.team == Team.Town:
                n_town += 1
            elif p.role.team == Team.Mafia:
                n_maf += 1

        # If there are no mafia left
        if not n_maf >= 1:
            raise GameEndException(m, winner=Team.Town)
        
        if not n_maf * 2 < n:
            raise GameEndException(m, winner=Team.Mafia)

    def to_day(self, m : GameState):
        # Collect stuns
        stuns = set()
        for p in m.players:
            if p.role == Role.STRIPPER and \
            p.id in m.round.targets and \
            m.round.targets[p.id] not in [None, PlayerID.NOTARGET]:
                stuns.add(m.round.targets[p.id])
                self.handle(Event.STUN, **locals())
        
        # TODO: Send out stun messages if rule?

        # Collect Saves
        saves = dict([(pat,doc) for doc,pat in m.round.targets.items() \
            if m.getPlayer(doc).role == Role.DOCTOR and not doc in stuns])

        # Check kill
        if m.round.mafia_target == PlayerID.NOTARGET:
            # TODO: Send Player.NOTARGET message
            logging.info("mafia Player.NOTARGET message")
        else:
            if m.round.mafia_target in saves:
                # TODO: Send No Kill message and savior messages
                logging.info("Doc savior messages")
                logging.info("mafia Player.NOTARGET message (save)")
            else:
                # TODO: Send Death messages
                logging.info("Kill message")
                try:
                   self.eliminate(m, m.round.mafia_target)
                except GameEndException as ge:
                    logging.info(f"Winner: {ge.winner}")
                    return
            
        # Investigate
        for cop in m.players:
            if cop.role == Role.COP and not cop in stuns and m.round.targets[cop] in m.players:
                logging.info(f"{cop} investigates {m.round.targets[cop]}")
        
        m.round.phase = Phase.DAY
                

    def check_to_day(self, m : GameState):
        if m.round.phase != Phase.NIGHT:
            raise TypeError(f"Cannot check_to_day if not Phase.NIGHT: {m.round.phase}")
        
        # Collect targeting role players
        targeting_players = [p.id for p in m.players if p.role.targeting]
        if all([tp in m.round.targets for tp in targeting_players]) and \
            m.round.mafia_target != None:
            
            self.to_day(m)
            return True
        return False

    def elect(self, m : GameState, voter : PlayerID, votee : PlayerID):
        logging.info(f"Election")

        if votee == PlayerID.NOTARGET:
            logging.info("Nobody elected")
        else:
            logging.info(f"{votee} elected")
            try:
                self.eliminate(m, votee)
            except GameEndException as ge:
                logging.info(f"Winner: {ge.winner}")
                return
        self.night(m)
    
    def night(self, m):
        m.round.phase = Phase.NIGHT

    def check_vote_thresh(self, m : GameState, target : PlayerID):
        if m.round.phase != Phase.DAY:
            raise TypeError(f"Cannot check_to_day if not Phase.DAY: {m.round.phase}")
        
        # Collect number of votes for each candidate?
        vote_count = 0
        for (voter,votee) in m.round.votes.items():
            if votee == target:
                vote_count += 1
        n = len(m.players)
        if target == PlayerID.NOTARGET:
            thresh = (n+1) // 2
        else:
            thresh = (n // 2) + 1
        return thresh, vote_count
