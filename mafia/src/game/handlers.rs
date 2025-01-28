use crate::prelude::*;
use async_trait::async_trait;

#[async_trait]
pub trait ActionHandler<P> {
    fn init(&mut self, game: &Game);
    async fn recv_action(&mut self) -> Option<Action<P>>;
    async fn resp_action(&mut self, result: Result<(), Error>);
    async fn update_status(&mut self, state: &State);
}

#[async_trait]
pub trait EventHandler {
    fn init(&mut self, game: &Game);
    async fn handle_event(&mut self, event: Event);
}
