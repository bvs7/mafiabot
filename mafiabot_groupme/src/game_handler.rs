use crate::prelude::*;

use crate::app::AppState;

use tokio::sync::TryLockError;
use tokio::task::{Id, JoinHandle};
use tracing::event;

mod event_handler;
use event_handler::EventHandler;

// TODO: make the responder actually send the response, and have the response info for that

#[derive(Debug)]
pub struct GameHandler {
    pub game: Game<u64>,
    event_task: JoinHandle<()>,
    pub main_chat_id: GroupId,
    pub mafia_chat_id: GroupId,
}

// TODO: store lobby id?
impl GameHandler {
    pub async fn new(
        app_state: Arc<AppState>,
        members: Vec<groupme::Member>,
        rules: Rules,
        lobby_id: GroupId,
    ) -> Self {
        let players: Vec<Pid> = members.iter().map(|m| W(m.user_id).into()).collect();
        let (game, event_rx) = Game::new(players, rules);
        let game_id = game.id();
        let main_chat_id = app_state.create_group(format!("MAIN CHAT #{game_id}")).await;
        let mafia_chat_id = app_state.create_group(format!("MAFIA CHAT #{game_id}")).await;

        // Get mafia members
        let state = game.get_state();
        let is_mafia = |pid: Pid| -> bool { state.players().get_role(pid).is_mafia() };
        let mafia_members: Vec<groupme::Member> =
            members.iter().filter(|m| is_mafia(W(m.user_id).into())).cloned().collect();

        // Add members to chats
        let mut w_groups = app_state.groups.write().await;
        w_groups.get_mut(&main_chat_id).unwrap().add_members(members).await;
        w_groups.get_mut(&mafia_chat_id).unwrap().add_members(mafia_members).await;
        drop(w_groups);

        let event_handler = EventHandler::new(
            game.id(),
            event_rx,
            lobby_id.clone(),
            main_chat_id.clone(),
            mafia_chat_id.clone(),
            app_state.clone(),
        );

        let event_task = tokio::spawn(event_handler.run());

        Self { game, event_task, main_chat_id, mafia_chat_id }
    }

    pub async fn send_action(&self, action: Action<u64>) -> Result<(), GameError> {
        self.game.send_action(action).await
    }

    pub fn id(&self) -> GameId {
        self.game.id()
    }

    pub async fn stop(self) {
        self.game.stop().await;
        match self.event_task.await {
            Ok(_) => info!("Event task stopped"),
            Err(e) => error!("Error stopping event task: {:?}", e),
        }
    }

    pub async fn parse_main_chat_cmd(
        &self,
        user_id: UserId,
        words: Vec<String>,
        attachments: Vec<Attachment>,
        app_state: &Arc<AppState>,
    ) {
        // Check for vote
        let mut words = words.into_iter();
        let Some(command) = words.next() else {
            return;
        };
        if command == "/vote" {
            // Check for a mention
            let mut ballot: Option<Option<u64>> = None;
            for attachment in attachments {
                if let Attachment::Mentions { user_ids } = attachment {
                    if let Some(id) = user_ids.first() {
                        ballot = Some(Some(u64::from(*id)));
                        break;
                    };
                }
            }
            if ballot.is_none() {
                match words.next().as_ref().map(|s| s.as_str()) {
                    Some("none") => ballot = None,
                    Some("nokill") => ballot = Some(None),
                    _ => {}
                }
            }
            // Send vote
        }
    }
}
