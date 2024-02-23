use crate::base::{Choice, ID};
use crate::core::PhaseKind;
use crate::events::Event;
use crate::roles::RoleKind;

use std::sync::mpsc::SendError;

#[derive(Debug)]
pub enum CoreError<PID: ID> {
    InvalidPhase {
        actual: PhaseKind,
        expected: PhaseKind,
    },
    InvalidPlayer {
        player: PID,
    },
    ExpectedTargetingRole {
        role: RoleKind,
    },
    ExpectedSchemingRole {
        role: RoleKind,
    },
    ExpectedCeleb {
        actual: RoleKind,
    },
    ExpectedElection {
        candidate: Choice<PID>,
    },
    EventSendError(SendError<Event<PID>>),
}

impl<PID: ID> From<SendError<Event<PID>>> for CoreError<PID> {
    fn from(e: SendError<Event<PID>>) -> Self {
        CoreError::EventSendError(e)
    }
}
