pub use serde::{Deserialize, Serialize};
pub use std::{collections::HashMap, sync::Arc};
pub use tokio::sync::{broadcast, mpsc, oneshot, watch, RwLock};
pub use tracing::{debug, error, info, trace, warn};

pub use crate::app::AppState;
pub use crate::game_handler::GameHandler;

pub use groupme::{
    self, api,
    subscriber::{Attachment, Data, PushWebSocketServer},
    GroupId, MessageId, UserId,
};
pub use mafia::{
    game::{Action, Error as GameError, Event, EventRx, Game, GameId},
    rolegen::RoleGen,
    rules::Rules,
    state::{phase::PhaseKind, State},
    Pid, Role, RoleKind, Team,
};

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
