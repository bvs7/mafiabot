use std::sync::Arc;

use tokio::{
    sync::{Notify, RwLock, RwLockReadGuard},
    task::JoinHandle,
};
use tracing::{debug, info, warn};

use super::{ActionMsg, ActionRx, EventTx, Role, Rules, State};

#[derive(Debug)]
pub struct Game {
    pub state: Arc<RwLock<State>>,
    timer: Arc<Notify>,
    action_rx: ActionRx,
    event_tx: EventTx,
    quit: Arc<Notify>,
}

impl Game {
    pub fn new<'a>(
        game_id: u64,
        registry: impl IntoIterator<Item = &'a (u64, Role)>,
        rules: Rules,
        action_rx: ActionRx,
        event_tx: EventTx,
        quit: Arc<Notify>,
    ) -> Self {
        Self {
            state: Arc::new(RwLock::new(State::new(game_id, registry, rules))),
            timer: Arc::new(Notify::new()),
            action_rx,
            event_tx,
            quit,
        }
    }

    pub async fn handle_action(&self, (action, responder): ActionMsg) {
        debug!("Got Action: {:?}", action);
        let result = {
            let rstate = self.state.read().await;
            rstate.validate_action(&action)
        };
        let valid = result.is_ok();
        responder.send(result).unwrap();
        if valid {
            let mut wstate = self.state.write().await;
            wstate.handle_action(action, &self.event_tx).await;
        }
    }

    pub async fn handle_timer(&self) {
        let mut wstate = self.state.write().await;
        wstate.handle_timer(&self.event_tx).await;
    }

    /// Run the game loop, receiving actions, watching timers
    pub async fn run(mut self) {
        debug!("Starting Game Loop");
        loop {
            debug!("Game Loop Beginning");
            tokio::select! {
                _ = self.quit.notified() => {
                    info!("Got Quit Notification");
                    break;
                },
                a = self.action_rx.recv() => if let Some(act) = a {
                    self.handle_action(act).await
                } else {
                    warn!("self.action_rx.recv() closed? quitting");
                    break;

                },
                _ = self.timer.notified() => self.handle_timer().await,
            }
        }
    }

    /// Spawn a tokio task to run this game, consuming it, then return.
    pub async fn start(self) -> JoinHandle<()> {
        tokio::spawn(self.run())
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use tokio::sync::{broadcast, mpsc};
    use tracing_test::traced_test;

    use crate::engine::interface::{ActionTx, EventRx};

    use super::*;

    fn basic_game() -> (Game, ActionTx, EventRx, Arc<Notify>) {
        let (a_tx, a_rx) = mpsc::channel(100);
        let (e_tx, e_rx) = broadcast::channel(100);
        let quit = Arc::new(Notify::new());

        let registry: Vec<(u64, Role)> = vec![
            (1, Role::TOWN),
            (2, Role::COP),
            (3, Role::DOCTOR),
            (4, Role::MAFIA),
        ];

        let game = Game::new(0, &registry, Rules {}, a_rx, e_tx, quit.clone());

        return (game, a_tx, e_rx, quit);
    }

    #[tokio::test]
    async fn basic() {
        assert!(true);
    }

    #[traced_test]
    #[tokio::test]
    async fn quit() {
        let (game, a_tx, _, quit) = basic_game();
        // Start game
        let game_join = tokio::spawn(async move {
            debug!("Run Game Thread");
            game.run().await
        });

        drop(a_tx);

        // quit.notify_one();

        tokio::select! {
            _ = game_join => {},
            _ = tokio::time::sleep(Duration::from_millis(10000)) => {panic!("Failed to join game");}
        }
    }
}
