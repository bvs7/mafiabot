use crate::prelude::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid Player id {pid}")]
    InvalidPlayer { pid: u64 },
    #[error("Dead Player")]
    DeadPlayer,
}
