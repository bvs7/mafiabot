enum Action {}
enum Event {}

#[derive(Debug)]
struct GameId(u64);

impl std::fmt::Display for GameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct UserId(u64);
struct GroupId(u64);

struct Member {
    nickname: String,
    user_id: UserId,
}

struct GroupMeGroup {
    group_id: GroupId,
}
impl GroupMeGroup {
    async fn new(name: &str) -> Self {
        Self { group_id: GroupId(0) }
    }

    async fn from_group_id(group_id: GroupId) -> Self {
        // TODO:
        Self { group_id }
    }
}

#[derive(Debug)]
enum ActorTxMsg {
    CreateGame { game_id: GameId, status: watch::Receiver<Status> },
}

type ActorTx = broadcast::Sender<ActorTxMsg>;
type ActorRx = broadcast::Receiver<ActorTxMsg>;

#[derive(Debug)]
struct Status {}
struct Game {
    id: GameId,
    event_tx: mpsc::Sender<Event>,
}
impl Game {
    fn new(event_tx: mpsc::Sender<Event>) -> Self {
        Self { id: GameId(0), event_tx }
    }
}

type GameStatusTx = watch::Sender<Status>;
type GameStatusRx = watch::Receiver<Status>;

// MOVE EVERTHING ABOVE HERE
use tokio::sync::{broadcast, mpsc, watch};
use tracing::event;

struct GameActor {
    game: Game,
    lobby_chat_id: GroupId,
    action_rx: mpsc::Receiver<Action>,
    game_status_tx: watch::Sender<Status>,
    actor_tx: broadcast::Sender<ActorTxMsg>,
    main_chat: GroupMeGroup,
    mafia_chat: GroupMeGroup,
}

impl GameActor {
    async fn create(
        members: Vec<Member>,
        lobby_chat_id: GroupId,
        actor_tx: tokio::sync::broadcast::Sender<ActorTxMsg>,
    ) {
        let (action_tx, action_rx) = tokio::sync::mpsc::channel(100);
        let (game_status_tx, game_status_rx) = tokio::sync::watch::channel(Status {});
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);

        let game = Game::new(event_tx);

        let main_chat = GroupMeGroup::new(format!("MAIN CHAT #{}", game.id).as_str()).await;
        let mafia_chat = GroupMeGroup::new(format!("MAFIA CHAT #{}", game.id).as_str()).await;

        let game_actor = GameActor {
            game,
            lobby_chat_id,
            action_rx,
            game_status_tx,
            actor_tx,
            main_chat,
            mafia_chat,
        };

        tokio::spawn(async move {
            game_actor.update_loop().await;
        });

        tokio::spawn(async move {
            Self::event_listener(event_rx, game_status_rx).await;
        });
    }

    /// Has access to game, and so can update state from that.
    async fn update_loop(mut self) {
        let game_id = self.game.id;
        let status = self.game_status_tx.subscribe();
        self.actor_tx.send(ActorTxMsg::CreateGame { game_id, status }).unwrap();

        let mut alarm_time: Option<DateTime<Local>> = None;
        loop {
            let action = self.action_rx.recv().await;
            // Do action
        }
    }

    async fn event_listener(
        mut event_rx: tokio::sync::mpsc::Receiver<Event>,
        game_status_tx: tokio::sync::watch::Receiver<Status>,
    ) {
        loop {
            let event = event_rx.recv().await;
            let status = game_status_tx.borrow();
            // Parse event
            // Call api functions to send things out
        }
    }
}
