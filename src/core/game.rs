use super::{Contracts, GameRules, PIDs, Phase, RoleGen};

// Entrants + RoleGen -> Players + Contracts

pub struct Game {
    game_id: usize,
    phase: Phase,
    contracts: Contracts,
    entrants: PIDs,
    rolegen: RoleGen,
    rules: GameRules,
}
