use std::fmt::Display;

use super::*;

#[derive(Debug)]
pub enum InvalidActionError {
    InvalidPhase {
        expected: PhaseKind,
        found: Phase,
    },
    InvalidAction {
        action: ActionKind,
        phase: PhaseKind,
    },
    PlayerNotFound {
        pid: PID,
    },
    InvalidRole {
        role: Role,
        action: ActionKind,
    },
    NoGame,
    InvalidTargetText {
        text: String,
    },
    InvalidTarget {
        target: Pidx,
    },
}

impl Display for InvalidActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error with command: ")?;
        match self {
            Self::InvalidPhase { expected, found } => {
                write!(
                    f,
                    "Invalid Phase (expected {:?}, found {:?}",
                    expected, found
                )
            }
            Self::InvalidAction { action, phase } => {
                write!(f, "Invalid Action ({:?}) for Phase ({:?})", action, phase)
            }
            Self::PlayerNotFound { pid } => {
                write!(f, "Player with UserID {:?} not found", pid)
            }
            Self::InvalidRole { role, action } => {
                write!(f, "Invalid Role ({:?}) for Action ({:?})", role, action)
            }
            Self::NoGame => {
                write!(f, "No Game")
            }
            Self::InvalidTargetText { text } => {
                write!(f, "Invalid Target: {}", text)
            }
            Self::InvalidTarget { target } => {
                write!(f, "Invalid Target: {}", target)
            }
        }
    }
}
impl std::error::Error for InvalidActionError {}
