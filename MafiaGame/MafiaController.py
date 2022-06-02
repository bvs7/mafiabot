
from typing import List, Tuple

import logging
logging.basicConfig(level=logging.DEBUG)

from .MafiaUtil import *
from .MafiaRules import *
from .MafiaState import *

class MafiaController:
    
    async def start(chat:ChatHandle, game_number:int,  
                    role_list:List[Tuple[PlayerID,Role]], rules:RuleSet):

        players = set()
        for p_id,role in role_list:
            players.add(Player(p_id, role))
        return GameState(chat, game_number, players=players, rules=rules)

    async def vote(m:GameState, voter:PlayerID, votee:PlayerID):
        if not m.round.phase == Phase.DAY:
            raise MafiaFormatError("Can only vote during Day")
        m.round.votes[voter] = votee

        thresh, count = MafiaController.check_vote_thresh(m, votee)

        # Resp
        
        await MafiaController.check_to_night(m)

    def eliminate(m:GameState, p_id:PlayerID):
        p = m.getPlayer(p_id)
        # TODO: Check for victory
        # logging.info(f"{p_id} died")
        phase_id = Event.getPhaseID(m.lobby_chat, m.game_number, m.round.day, m.round.phase)

        m.players.remove(p)

        MafiaController.check_win(m)

    def check_win(m:GameState):
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
        
        if not n_maf * 2 <= n_town:
            raise GameEndException(m, winner=Team.Mafia)

    def to_day(m : GameState):
        # Collect stuns
        stuns = set()
        for p in m.players:
            if p.role == Role.STRIPPER and \
            p.id in m.round.targets and \
            m.round.targets[p.id] not in [None, Player.NOTARGET]:
                stuns.add(m.round.targets[p.id])
        
        # TODO: Send out stun messages if rule
        logging.info("Block messages")

        # Collect Saves
        saves = dict([(pat,doc) for doc,pat in m.round.targets.items() \
            if m.getPlayer(doc).role == Role.DOCTOR and not doc in stuns])

        # Check kill
        if m.round.mafia_target == Player.NOTARGET:
            # TODO: Send Player.NOTARGET message
            logging.info("mafia Player.NOTARGET message")
        else:
            if m.round.mafia_target in saves:
                # TODO: Send No Kill message and savior messages
                logging.info("Doc savior messages")
                logging.info("mafia Player.NOTARGET message")
            else:
                # TODO: Send Death messages
                logging.info("Kill message")
                try:
                    MafiaController.eliminate(m, m.round.mafia_target)
                except GameEndException as ge:
                    logging.info(f"Winner: {ge.winner}")
                    return
            
        # Investigate
        for cop in m.players:
            if cop.role == Role.COP and not cop in stuns:
                logging.info(f"{cop} investigates {m.round.targets[cop]}")
        
        m.round.phase = Phase.DAY
                

    def check_to_day(m : GameState):
        if m.round.phase != Phase.NIGHT:
            raise TypeError(f"Cannot check_to_day if not Phase.NIGHT: {m.round.phase}")
        
        # Collect targeting role players
        targeting_players = [p.id for p in m.players if p.role.targeting]
        if all([tp in m.round.targets for tp in targeting_players]) and \
            m.round.mafia_target != None:
            
            MafiaController.to_day(m)
            return True
        return False

    def to_night(m : GameState, votee : PlayerID):
        logging.info(f"Election")

        if votee == Player.NOTARGET:
            logging.info("Nobody elected")
        else:
            logging.info(f"{votee} elected")
            try:
                MafiaController.eliminate(m, votee)
            except GameEndException as ge:
                logging.info(f"Winner: {ge.winner}")
                return

        m.round.phase = Phase.NIGHT

    def check_vote_thresh(m : GameState, target : PlayerID):
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

    async def check_to_night(m : GameState, target : PlayerID = None):
        if m.round.phase != Phase.DAY:
            raise TypeError(f"Cannot check_to_day if not Phase.DAY: {m.round.phase}")
        
        # Collect number of votes for each candidate?
        vote_counts = {}
        for (voter,votee) in m.round.votes.items():
            if not votee in vote_counts:
                vote_counts[votee] = 0
            vote_counts[votee] += 1
        nk_count = 0
        if Player.NOTARGET in vote_counts:
            nk_count = vote_counts[Player.NOTARGET]
        vote_counts = sorted(list(vote_counts.items()), key=lambda x: x[1], reverse=True)
        n = len(m.players)
        thresh = (n // 2) + 1
        nk_thresh = (n+1) // 2

        if nk_count >= nk_thresh:
            MafiaController.to_night(m, Player.NOTARGET)
            return True

        if len(vote_counts) > 0:
            top_votee, n_votes = vote_counts[0]
            if n_votes >= thresh:
                MafiaController.to_night(m, top_votee)
                return True

        return False
