use crate::base::ID;
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
    EventSendError(SendError<Event<PID>>),
}
