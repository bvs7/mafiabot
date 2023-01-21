/// A set of rules that change how the game can be played.
use std::default::Default;

pub struct GameRules {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// At the start of the game, role info revealed includes...
pub enum StartInfo {
    /// Nothing
    None,
    #[default]
    /// Number of Mafia Aligned players vs number of Not Mafia Aligned players
    Mafia,
    /// Number of Town Aligned, Mafia Aligned, and Rogue Unaligned players
    Team,
    /// Number of each Role
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// The game starts in Night Phase...
pub enum StartNight {
    /// Always
    Always,
    #[default]
    /// If there are an even number of players
    Even,
    /// Never
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// Upon a player's death, the role info revealed includes...
pub enum DeathReveal {
    /// Nothing
    None,
    /// Mafia Aligned or Not Mafia Aligned
    Mafia,
    #[default]
    Team,
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElectionInfo {
    #[default]
    Public,
    Revealed,
    Count,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElectionProcess {
    #[default]
    Dynamic,
    Static,
    SingleStatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Investigation {
    #[default]
    Mafia,
    Team,
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StripNotify {
    #[default]
    Useful,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SaveSelf {
    #[default]
    Always,
    Stun,
    Never,
}
