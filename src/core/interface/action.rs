// Module: core::interface::action
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    Vote,
    Reveal,
    Target,
    Mark,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Vote { voter: PID, ballot: Option<Choice> },
    Reveal { celeb: PID },
    Target { actor: PID, target: Choice },
    Mark { killer: PID, mark: Choice },
}
impl Action {
    pub fn kind(&self) -> ActionKind {
        match self {
            Action::Vote { .. } => ActionKind::Vote,
            Action::Reveal { .. } => ActionKind::Reveal,
            Action::Target { .. } => ActionKind::Target,
            Action::Mark { .. } => ActionKind::Mark,
        }
    }
}
