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
/// Upon a player's death, the info revealed is...
pub enum DeathReveal {
    /// Nothing
    None,
    /// Mafia Aligned or Not Mafia Aligned
    Mafia,
    #[default]
    /// Team Alignment
    Team,
    /// Role
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElectionInfo {
    #[default]
    /// Votes are announced immediately in public
    Public,
    /// Votes are cast in private and revealed at the end of the day
    Revealed,
    /// Votes are cast in private the number of votes for each option
    /// are revealed at the end of the day
    Count,
    /// Votes are cast in private and never revealed
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElectionProcess {
    #[default]
    /// Votes can be cast at any time. When a majority is reached, it causes an election
    Dynamic,
    /// There are scheduled election end times. If a majority exists at one of those
    /// times, it causes an election. If not, day continues
    Static,
    /// These is one scheduled election end time. If a majority exists at that time,
    /// is causes an election. If not, day ends with no election.
    SingleStatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// The role a COP sees upon investigating someone
pub enum Investigation {
    #[default]
    /// Mafia Aligned or Not Mafia Aligned
    Mafia,
    /// Team Alignment
    Team,
    /// Role
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// Upon a STRIPPER stripping a target, the target learns they were stripped...
pub enum StripNotify {
    #[default]
    /// When it has a useful effect. So if they were a COP that targeted someone,
    /// a DOCTOR who would have had a successful save, or a CELEB when they try to reveal
    /// Basically, when someone would otherwise be confused about why their action didn't work
    Useful,
    /// Target is notified immediately when they are stripped
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// When can a DOCTOR save themself?
pub enum SaveSelf {
    #[default]
    /// A DOCTOR can always target themself
    Always,
    /// When the DOCTOR saves themself, they will be stunned the following night and
    /// will be unable to save anyone
    Stun,
    /// A DOCTOR can never target themself
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// Upon a successful save, where one or more DOCTORS save the Mafia killer's mark...
pub enum SaveInfo {
    /// The mark is publicly reveal at the start of the Day
    Public,
    /// The DOCTOR(s) and mark are informed privately
    Private,
    #[default]
    /// The DOCTOR(s) are informed privately
    Doctor,
    /// No one is informed
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// Upon Eleting and IDIOT...
pub enum IdiotElect {
    /// Nothing special happens
    None,
    #[default]
    /// A Dusk Phase occurs, where the IDIOT picks one player out of those who voted for
    /// them. This player and the IDIOT are both eliminated.
    Dusk,
    /// The IDIOT is eliminated, but a new DAY phase begins
    Day,
    /// Everyone who voted for the IDIOT is stunned and cannot act during the night
    Stun,
    /// Everyone who voted for the IDIOT is eliminated, along with the IDIOT.
    Cull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// When a living GUARD's charge dies...
pub enum GuardContract {
    /// The GUARD dies
    Retire,
    #[default]
    /// The GUARD becomes tasked with causing the death of whoever caused the charge's death.
    /// If it was themself, they become an IDIOT
    Refocus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
/// When a living AGENT's target dies...
pub enum AgentContract {
    /// The AGENT dies
    Retire,
    #[default]
    /// The AGENT becomes tasked with causing the death of whoever caused the target's death.
    /// If it was themself, they become a SURVIVOR
    Refocus,
}
