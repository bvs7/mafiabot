
from MafiaGameState import *


def eliminate(m:MafiaGameState, p_id:PlayerID):
    p = m.getPlayer(p_id)
    # TODO: Check for victory
    logging.info(f"{p_id} died")

    m.players.remove(p)

    check_win(m)

def check_win(m:MafiaGameState):
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
    


def to_day(m : MafiaGameState):
    # Collect stuns
    stuns = set()
    for p in m.players:
        if p.role == Role.STRIPPER and \
           p.id in m.round.targets and \
           m.round.targets[p.id] not in [None, NOTARGET]:
            stuns.add(m.round.targets[p.id])
    
    # TODO: Send out stun messages if rule
    logging.info("Block messages")

    # Collect Saves
    saves = dict([(pat,doc) for doc,pat in m.round.targets.items() \
        if m.getPlayer(doc).role == Role.DOCTOR and not doc in stuns])

    # Check kill
    if m.round.mafia_target == NOTARGET:
        # TODO: Send NOTARGET message
        logging.info("mafia NOTARGET message")
    else:
        if m.round.mafia_target in saves:
            # TODO: Send No Kill message and savior messages
            logging.info("Doc savior messages")
            logging.info("mafia NOTARGET message")
        else:
            # TODO: Send Death messages
            logging.info("Kill message")
            try:
                eliminate(m, m.round.mafia_target)
            except GameEndException as ge:
                logging.info(f"Winner: {ge.winner}")
                return
        
    # Investigate
    for cop in m.players:
        if cop.role == Role.COP and not cop in stuns:
            logging.info(f"{cop} investigates {m.round.targets[cop]}")
    
    m.round.phase = Phase.DAY
            

def check_to_day(m : MafiaGameState):
    if m.round.phase != Phase.NIGHT:
        raise TypeError(f"Cannot check_to_day if not Phase.NIGHT: {m.round.phase}")
    
    # Collect targeting role players
    targeting_players = [p.id for p in m.players if p.role.targeting]
    if all([tp in m.round.targets for tp in targeting_players]) and \
        m.round.mafia_target != None:
        
        to_day(m)
        return True
    return False

def to_night(m : MafiaGameState, votee : PlayerID):
    logging.info(f"Election")

    if votee == NOTARGET:
        logging.info("Nobody elected")
    else:
        logging.info(f"{votee} elected")
        try:
            eliminate(m, votee)
        except GameEndException as ge:
            logging.info(f"Winner: {ge.winner}")
            return

    m.round.phase = Phase.NIGHT

def check_to_night(m : MafiaGameState):
    if m.round.phase != Phase.DAY:
        raise TypeError(f"Cannot check_to_day if not Phase.DAY: {m.round.phase}")
    
    # Collect number of votes for each candidate?
    vote_counts = {}
    for (voter,votee) in m.round.votes.items():
        if not votee in vote_counts:
            vote_counts[votee] = 0
        vote_counts[votee] += 1
    nk_count = 0
    if NOTARGET in vote_counts:
        nk_count = vote_counts[NOTARGET]
    vote_counts = sorted(list(vote_counts.items()), key=lambda x: x[1], reverse=True)
    n = len(m.players)
    thresh = (n // 2) + 1
    nk_thresh = (n+1) // 2

    if nk_count >= nk_thresh:
        to_night(m, NOTARGET)
        return True

    if len(vote_counts) > 0:
        top_votee, n_votes = vote_counts[0]
        if n_votes >= thresh:
            to_night(m, top_votee)
            return True

    return False
