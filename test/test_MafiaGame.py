
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



