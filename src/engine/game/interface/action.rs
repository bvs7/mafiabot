use crate::engine::game::*;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum Action {
    Vote { voter: u64, ballot: Option<Option<u64>> },
    Target { actor: u64, choice: Option<u64> },
    Scheme { killer: u64, mark: Option<u64> },
    Reveal { actor: u64 },
}

#[derive(Debug, Clone)]
pub enum Error {
    InvalidPhase { expected: PhaseKind, actual: PhaseKind },
    InvalidPlayer { pid: u64 },
    DeadPlayer { pid: u64 },
    ExpectedTargetingRole { actual: RoleKind },
    ExpectedSchemingRole { actual: RoleKind },
    ExpectedCeleb { actual: RoleKind },
    IneffectiveAction,
    InvalidTarget { idx: usize },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidPhase { expected, actual } => {
                write!(f, "Invalid Phase. Expected {expected} but got {actual}")
            }
            Error::InvalidPlayer { pid } => write!(f, "Invalid player id: {pid}"),
            Error::DeadPlayer { pid } => write!(f, "Player is dead: {pid}"),
            Error::ExpectedTargetingRole { actual } => {
                write!(f, "Expected targing role, got {actual}")
            }
            Error::ExpectedSchemingRole { actual } => {
                write!(f, "Expected scheming role, got {actual}")
            }
            Error::ExpectedCeleb { actual } => {
                write!(f, "Expected celeb, got {actual}")
            }
            Error::IneffectiveAction => {
                write!(f, "This action would not have any effect")
            }
            Error::InvalidTarget { idx } => {
                write!(f, "Invalid target index: {idx}")
            }
        }
    }
}

pub type ActionResponder = tokio::sync::oneshot::Sender<Result<(), Error>>;
pub type ActionMsg = (Action, ActionResponder);
pub type ActionRx = tokio::sync::mpsc::Receiver<ActionMsg>;
pub type ActionTx = tokio::sync::mpsc::Sender<ActionMsg>;
