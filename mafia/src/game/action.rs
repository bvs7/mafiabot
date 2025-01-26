use crate::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Action<P> {
    Vote { voter: P, ballot: Option<Option<P>> },
    Target { actor: P, choice: Option<P> },
    Scheme { killer: P, mark: Option<P> },
    Reveal { actor: P },
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid Player id {pid}")]
    InvalidPlayer { pid: u64 },
    #[error("Dead Player")]
    DeadPlayer,
    #[error("Invalid Phase. Expected {expected} but got {actual}")]
    InvalidPhase { expected: PhaseKind, actual: PhaseKind },
    #[error("Expected targing role, got {actual}")]
    ExpectedTargetingRole { actual: RoleKind },
    #[error("Expected scheming role, got {actual}")]
    ExpectedSchemingRole { actual: RoleKind },
    #[error("Expected celeb, got {actual}")]
    ExpectedCeleb { actual: RoleKind },
    #[error("This action would not have any effect")]
    IneffectiveAction,
    #[error("Invalid target index: {idx}")]
    InvalidTarget { idx: usize },
}
