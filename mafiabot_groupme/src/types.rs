use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct W<T>(pub T);

impl From<W<UserId>> for Pid {
    fn from(value: W<UserId>) -> Self {
        Self(value.0 .0)
    }
}
impl From<W<Pid>> for UserId {
    fn from(value: W<Pid>) -> Self {
        Self(value.0 .0)
    }
}
