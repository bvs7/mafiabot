use crate::{controller::Command, discord::UserID};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    Vote,
    Retract,
    Reveal,
    Target,
    Mark,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action<U: RawPID> {
    Vote { voter: U, ballot: Option<Choice<U>> },
    Reveal { celeb: U },
    Target { actor: U, target: Choice<U> },
    Mark { killer: U, mark: Choice<U> },
}
impl<U: RawPID> Action<U> {
    pub fn kind(&self) -> ActionKind {
        match self {
            Action::Vote { .. } => ActionKind::Vote,
            Action::Reveal { .. } => ActionKind::Reveal,
            Action::Target { .. } => ActionKind::Target,
            Action::Mark { .. } => ActionKind::Mark,
        }
    }
}

impl TryFrom<Command> for Action<UserID> {
    type Error = ();
    fn try_from(value: Command) -> Result<Self, Self::Error> {
        match value {
            Command::Vote { voter, ballot } => Ok(Action::Vote { voter, ballot }),
            Command::Reveal { celeb } => Ok(Action::Reveal { celeb }),
            Command::Target { actor, target } => Ok(Action::Target { actor, target }),
            Command::Mark { killer, mark } => Ok(Action::Mark { killer, mark }),
            _ => Err(()),
        }
    }
}
