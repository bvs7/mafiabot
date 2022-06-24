
import pytest
import logging

logging.basicConfig(level=logging.DEBUG)

from MafiaGame import *


@pytest.fixture
def mafia_game():
    m = GameState(
        lobby_chat="lobby", game_number=10,
        main_chat="main", mafia_chat="mafia",
    )

    p1 = Player(1, Role.TOWN)
    p2 = Player(2, Role.MAFIA)
    p3 = Player(3, Role.GUARD, 1)

    m.players.update([p1,p2,p3])

    m.round.set_phase(Phase.DAY, votes={1:2})

    m.rules.startNight = StartNight.ODD

    return m

def test_vote(mafia_game : GameState):
    m = GameState.load("test/test_data/vote1_1.maf")

    assert len(m.players) == 7

    th = QueueTestEventHandler(handler_on_empty=PrintEventHandler())
    ctrl = MafiaController(handler=th)

    th.queue(Event.VOTE)
    th.queue(Event.VOTE, {"voter":3, "votee":1, "thresh":4, "count":2})

    ctrl.vote(m,2,1)
    ctrl.vote(m,3,1)
    ctrl.vote(m,4,1)

    assert m.round.phase == Phase.DAY

    ctrl.vote(m,4,PlayerID.NOTARGET)
    ctrl.vote(m,3,PlayerID.NOTARGET)
    ctrl.vote(m,2,PlayerID.NOTARGET)

    assert m.round.phase == Phase.DAY

    ctrl.vote(m, 1, PlayerID.NOTARGET)

    assert m.round.phase == Phase.NIGHT

    m.round.set_phase(Phase.DAY)

    assert m.round.phase == Phase.DAY

    ctrl.vote(m,2,1)
    ctrl.vote(m,3,1)
    ctrl.vote(m,4,1)
    ctrl.vote(m,5,1)

    assert m.round.phase == Phase.NIGHT
    assert not 1 in m.players

    m.round.set_phase(Phase.DAY)

    ctrl.vote(m,4,PlayerID.NOTARGET)
    ctrl.vote(m,3,PlayerID.NOTARGET)
    ctrl.vote(m,2,PlayerID.NOTARGET)

    assert m.round.phase == Phase.NIGHT

    m.round.set_phase(Phase.DAY)

    ctrl.vote(m,2,5)
    ctrl.vote(m,3,5)
    ctrl.vote(m,4,5)

    assert m.round.phase == Phase.DAY

    ctrl.vote(m,6,5)
    
    assert m.round.phase == Phase.NIGHT

def test_target():
    m = GameState.load("test/test_data/target1_1.maf")

    th = QueueTestEventHandler(handler_on_empty=PrintEventHandler())

    ctrl = MafiaController(handler=th)

    ctrl.target(m, 2, 1)
    ctrl.target(m, 3, 1)
    ctrl.target(m, 4, 1)
    ctrl.target(m, 5, 1)

    assert m.round.phase == Phase.NIGHT

    th.queue(Event.MAFIA_TARGET)
    th.queue(Event.STUN)

    ctrl.mafia_target(m, 6, 1)

    assert m.round.phase == Phase.DAY
    assert len(m.players) == 7

    m.round.phase = Phase.NIGHT

    ctrl.mafia_target(m, 6, 1)

    ctrl.target(m, 3, 1)
    ctrl.target(m, 4, 1)
    ctrl.target(m, 5, 3)
    ctrl.target(m, 2, 1)

    assert m.round.phase == Phase.DAY
    assert len(m.players) == 6

    m.round.phase = Phase.NIGHT

    ctrl.mafia_target(m, 6, 3)

    ctrl.target(m, 3, 3)
    ctrl.target(m, 4, 2)
    ctrl.target(m, 5, 2)
    ctrl.target(m, 2, 5)

    assert m.round.phase == Phase.DAY
    assert len(m.players) == 6

    m.round.phase = Phase.NIGHT

    ctrl.mafia_target(m, 6, 3)

    ctrl.target(m, 3, 2)
    ctrl.target(m, 4, 2)
    ctrl.target(m, 5, 2)
    ctrl.target(m, 2, 5)

    assert m.round.phase == Phase.DAY
    assert len(m.players) == 5

    m.round.phase = Phase.NIGHT

    ctrl.mafia_target(m, 6, 2)

    ctrl.target(m, 4, 2)
    ctrl.target(m, 5, 2)

    try:
        ctrl.target(m, 2, 5)
    except GameEndException:
        pass

    assert m.round.phase == Phase.END

def test_rules():
    assert StartNight.default == StartNight.EVEN
    assert SavePublic.default == SavePublic.ANON
    assert DeathReveal.NONE != StartReveal.NONE
    assert Investigate.TEAM == Investigate.TEAM

    r1 = RuleSet(savePrivate=SavePrivate.DOCTOR)
    r2 = RuleSet()
    r2.savePrivate = SavePrivate.DOCTOR

    assert r1 == r2
    r2.startNight = StartNight.NEVER
    assert r1 != r2

def test_save_game(mafia_game : GameState):
    s = mafia_game.save("test/game.maf")
    m2 = GameState.load("test/game.maf")

    assert mafia_game == m2
    m2.rules.startNight = StartNight.default
    assert mafia_game != m2

    p = mafia_game.players.pop()
    s = p.save("test/player.maf")
    p2 = Player.load("test/player.maf")
    assert p == p2
    p3 = mafia_game.getPlayer(3)
    assert p != p3

    r = mafia_game.round
    s = r.save("test/round.maf")
    r2 = MafiaEncodable.load("test/round.maf")
    assert r == r2
    r.day += 2
    assert r != r2

    ph = mafia_game.round.phase
    s = ph.save("test/phase.maf")
    ph2 = Phase.load("test/phase.maf")
    ph3 = MafiaEncodable.load("test/phase.maf")
    assert ph == ph2
    assert ph == ph3
    assert ph != Phase.INIT

    t = Team.Town
    s = t.save("test/team.maf")
    t2 = Team.load("test/team.maf")
    assert t == t2
    assert t2 != Team.Rogue


def test_discordResponse(mafia_game:GameState):

    ctrl = MafiaController(handler = DiscordResponse(TestBot()))

    ctrl.vote(mafia_game, 2,1)
