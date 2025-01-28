use crate::prelude::*;
use async_trait::async_trait;

#[async_trait]
pub trait ActionHandler {
    type PID: Into<Pid> + Copy;
    async fn recv_action(&mut self) -> Option<Action<Self::PID>>;
    async fn resp_action(&mut self, result: Result<(), Error>);
    async fn update_status(&mut self, state: &State);
}

#[async_trait]
pub trait EventHandler {
    async fn handle_event(&mut self, event: Event);
}
