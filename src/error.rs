use crate::core::interface::Event;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Generic {0}")]
    Generic(String),

    #[error("mpsc::SendError")]
    MpscSendEventError(#[from] std::sync::mpsc::SendError<Event>),

    #[error("mpsc::RecvError")]
    MpscRecvActionError(#[from] std::sync::mpsc::RecvError),
}
