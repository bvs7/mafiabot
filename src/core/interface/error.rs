use std::fmt::Display;

use super::*;

#[derive(Debug)]
pub enum InvalidActionError<U: RawPID> {
    InvalidPhase {
        expected: PhaseKind,
        found: Phase<U>,
    },
    InvalidAction {
        action: ActionKind,
        phase: PhaseKind,
    },
    PlayerNotFound {
        pid: U,
    },
    InvalidRole {
        role: Role,
        action: ActionKind,
    },
}

impl<U: RawPID> Display for InvalidActionError<U> {
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
                write!(f, "Invalid Command ({:?}) for Phase ({:?})", action, phase)
            }
            Self::PlayerNotFound { pid } => {
                write!(f, "Player with UserID {:?} not found", pid)
            }
            Self::InvalidRole { role, action } => {
                write!(f, "Invalid Role ({:?}) for Command ({:?})", role, action)
            }
        }
    }
}
impl<U: RawPID> std::error::Error for InvalidActionError<U> {}
