use crate::prelude::*;

use crate::app::AppState;

use mafia::game;
use tokio::sync::TryLockError;
use tokio::task::JoinHandle;
use tracing::event;

// Helper types for the game handler

// TODO: make the responder actually send the response, and have the response info for that

#[derive(Debug)]
pub struct Responder {
    tx: oneshot::Sender<Result<(), GameError>>,
}
impl ActionResp for Responder {
    fn send(self, result: Result<(), GameError>) {
        self.tx.send(result).unwrap();
    }
}

#[derive(Debug)]
pub struct ActionMpsc {
    action_rx: mpsc::Receiver<(Action<u64>, Responder)>,
    status_tx: watch::Sender<Status>,
}
impl ActionQueue for ActionMpsc {
    type PID = u64;
    type Resp = Responder;

    fn recv(&mut self) -> Option<(Action<u64>, Responder)> {
        self.action_rx.blocking_recv()
    }
    fn update(&mut self, status: Status) {
        let _ = self.status_tx.send(status);
    }
}

#[derive(Debug)]
struct EventSender {
    event_tx: mpsc::UnboundedSender<Event>,
}
impl mafia::state::EventHandler for EventSender {
    fn handle(&mut self, event: Event) {
        match self.event_tx.send(event) {
            Ok(_) => (),
            Err(e) => error!("Error sending event: {:?}", e),
        }
    }
}

pub type Game = mafia::game::Game<ActionMpsc, EventSender>;

#[derive(Debug)]
pub struct GameHandler {
    game_id: GameId,
    action_tx: mpsc::Sender<(Action<u64>, Responder)>,
    status_rx: watch::Receiver<Status>,
    event_handle: JoinHandle<()>,
    run_handle: JoinHandle<Game>,
    main_chat_id: GroupId,
    mafia_chat_id: GroupId,
}

impl GameHandler {
    /// Create a new game
    pub async fn new(
        app_state: Arc<AppState>,
        members: Vec<groupme::Member>,
        rules: Rules,
    ) -> Self {
        let game_id = GameId::new().unwrap_or_default();

        let players: Vec<Pid> = members.iter().map(|m| Pid::from(u64::from(m.user_id))).collect();
        let main_chat_id = app_state.create_group(format!("MAIN CHAT #{game_id}")).await;
        let mafia_chat_id = app_state.create_group(format!("MAFIA CHAT #{game_id}")).await;

        let (action_tx, action_rx) = mpsc::channel(100);
        let (status_tx, status_rx) = watch::channel(Status::default());
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let action_queue = ActionMpsc { action_rx, status_tx };
        let event_sender = EventSender { event_tx };
        let game = Game::with_id(game_id, players, rules, action_queue, event_sender);

        let event_handler = EventHandler::new(
            game_id,
            event_rx,
            status_rx.clone(),
            main_chat_id.clone(),
            mafia_chat_id.clone(),
            app_state.clone(),
        );

        let event_handle = tokio::spawn(event_handler.run());

        let run_handle = tokio::task::spawn_blocking(move || game.run());
        Self {
            game_id,
            action_tx,
            status_rx: status_rx.clone(),
            event_handle,
            run_handle,
            main_chat_id,
            mafia_chat_id,
        }
    }

    pub async fn send_action(&self, action: Action<u64>) -> Result<(), GameError> {
        let (tx, rx) = oneshot::channel();
        let resp = Responder { tx };
        let _ = self.action_tx.send((action, resp));
        rx.await.unwrap()
    }

    pub fn id(&self) -> GameId {
        self.game_id
    }

    pub async fn stop(self) -> Game {
        drop(self.action_tx);
        let game = self.run_handle.await.unwrap();
        self.event_handle.await.unwrap();
        game
    }
}

use std::fmt::Write;

// Needs to know players/roles, right?
struct EventHandler {
    game_id: GameId,
    event_rx: mpsc::UnboundedReceiver<Event>,
    status_rx: watch::Receiver<Status>,
    main_chat_id: GroupId,
    mafia_chat_id: GroupId,
    players: HashMap<Pid, Role>, // Cached players TODO have status just have roles???
    names: HashMap<UserId, String>,
    app_state: Arc<AppState>,
}
impl EventHandler {
    fn new(
        game_id: GameId,
        event_rx: mpsc::UnboundedReceiver<Event>,
        status_rx: watch::Receiver<Status>,
        main_chat_id: GroupId,
        mafia_chat_id: GroupId,
        app_state: Arc<AppState>,
    ) -> Self {
        Self {
            game_id,
            event_rx,
            status_rx,
            main_chat_id,
            mafia_chat_id,
            players: HashMap::new(),
            names: HashMap::new(),
            app_state,
        }
    }

    async fn run(mut self) {
        loop {
            match self.event_rx.recv().await {
                Some(event) => match self.handle_event(event).await {
                    Ok(_) => continue,
                    Err(e) => error!("Error handling event: {:?}", e),
                },
                None => {
                    info!("Event Handler for game {} got None, closing", self.game_id);
                    break;
                }
            }
        }
    }

    // TODO: every once in a while, update names anyways
    async fn get_name(&mut self, pid: Pid) -> String {
        let user_id = UserId(u64::from(pid));
        for _ in 0..3 {
            if let Some(name) = self.names.get(&user_id) {
                return name.clone();
            } else {
                self.app_state.update_names(&self.main_chat_id).await;
                let new_names = self.app_state.get_names(&self.main_chat_id).await;
                self.names.extend(new_names.into_iter());
            }
        }
        warn!("Could not get name for pid: {}", pid);
        format!("(Player ID {pid})")
    }

    async fn create_start_msg(&mut self, role: Role) -> String {
        let mut msg = String::new();
        let m = &mut msg;
        write!(m, "Your Role is {}, ", role);
        write!(m, "you are {} aligned.\n", role.team());
        write!(m, "Use /help {} or /help {} for more info", role, role.team());
        match role {
            Role::GUARD(charge) | Role::AGENT(charge) => {
                write!(m, "Your charge is {}", self.get_name(charge).await);
            }
            _ => {}
        }
        msg
    }

    async fn option_msg(&mut self) -> String {
        let mut msg = String::new();
        let m = &mut msg;
        write!(m, "Choose a target:\n");
        let mut c = 'A';
        let players: Vec<_> = self.players.iter().map(|(pid, _)| *pid).collect();
        for pid in players {
            let name = self.get_name(pid).await;
            write!(m, "{}: {}\n", c, name);
            c = (c as u8 + 1) as char;
        }
        msg
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<(), std::fmt::Error> {
        let mut msg = String::new();
        let m = &mut msg;
        // TODO: every once in a while, just update players to be sure?
        match event {
            Event::Start { players, rules } => {
                let names = self.app_state.get_names(&self.main_chat_id).await;
                let names: Vec<_> = names
                    .into_iter()
                    .map(|(uid, name)| (Pid::from(u64::from(uid)), name))
                    .collect();
                for (pid, role) in players.iter() {
                    let user_id = UserId(u64::from(*pid));
                    let msg = self.create_start_msg(*role).await;
                    let _ = api::send_group_message(&self.main_chat_id, &msg);
                }

                // Send group chat messages
                write!(m, "Game {} begins!\nPlayers:", self.game_id);
                for (pid, _) in players.iter() {
                    let name = self.get_name(*pid).await;
                    write!(m, "\n  {}", name);
                }
                let mafia_msg = format!("Welcome to the Mafia Chat for game {}!", self.game_id);
                let _ = api::send_group_message(&self.mafia_chat_id, &mafia_msg).await;
            }
            Event::Day { day, counts } => {
                write!(m, "Day {} proceeds...\n", day);
                if let Some(count) = counts.get(&Team::Town) {
                    write!(m, " Town: {}\n", count);
                }
                if let Some(count) = counts.get(&Team::Mafia) {
                    write!(m, " Mafia: {}\n", count);
                }
                if let Some(count) = counts.get(&Team::Rogue) {
                    write!(m, " Rogue: {}\n", count);
                }
            }
            Event::Night { day, counts } => {
                write!(m, "Night {} falls...\n", day);
                let _ = api::send_group_message(&self.main_chat_id, &msg).await;
                let opt = self.option_msg().await;
                for (pid, role) in self.players.iter() {
                    if role.is_targeting() {
                        let _ = api::send_dm(UserId(u64::from(*pid)), &opt).await;
                    }
                }
            }
            Event::Eclipse { avenger, hammer, guilty } => {
                let avenger = self.get_name(avenger).await;
                write!(
                    m,
                    "The sky darkens as the moon eclipses the sun... {avenger} \
                    will /vote for one of those who voted, to follow them into the end!\n"
                );
            }
            Event::Vengeance { avenger, victim } => {
                let avenger = self.get_name(avenger).await;
                let victim = self.get_name(victim).await;
                let msg = format!("{avenger} has chosen {victim} to die with them!\n",);
            }
            // TODO: add thresh to ballot and former?
            Event::Vote { voter, ballot, former } => {
                let n = self.players.len();
                let thresh = n / 2 + 1;
                let pthresh = (n + 1) / 2;
                let voter = self.get_name(voter).await;
                if let Some((choice, count)) = ballot {
                    if let Some(pid) = choice {
                        let name = self.get_name(pid).await;
                        write!(m, "{voter} votes for {name} ({count}/{thresh})");
                    } else {
                        write!(m, "{voter} votes for peace.({count}/{pthresh})");
                    }
                } else {
                    write!(m, "{voter} retracts their vote.");
                }
                if let Some((choice, count)) = former {
                    if let Some(pid) = choice {
                        let name = self.get_name(pid).await;
                        write!(m, "\n({name} still has {count}/{thresh})");
                    } else {
                        write!(m, "\n(peace still has {count}/{pthresh})");
                    }
                }
            }
            Event::Reveal { celeb } => {
                write!(m, "{celeb} reveals, they are CELEB!\n",);
            }
            Event::Election { choice, hammer, voters } => {
                if let Some(pid) = choice {
                    let name = self.get_name(pid).await;
                    write!(m, "{name} is elected!");
                } else {
                    write!(m, "Nobody has been elected.");
                }
            }
            Event::Dawn => {
                write!(m, "Dawn breaks...");
            }
            Event::Eliminate { player, role, context } => {
                let name = self.get_name(player).await;
                self.players.remove(&player);
                let team = role.team();
                write!(m, "{name} was {team}!");
            }
            Event::Target { actor, choice } => {
                let mut dm = String::new();
                let m = &mut dm;
                if let Some(pid) = choice {
                    let name = self.get_name(pid).await;
                    write!(m, "You target {name}...");
                } else {
                    write!(m, "You target nobody...");
                }
                let _ = api::send_dm(UserId(u64::from(actor)), &dm).await;
            }
            Event::Scheme { killer, mark } => {
                let mut maf_msg = "".to_string();
                let m = &mut maf_msg;
                let actor = self.get_name(killer).await;
                if let Some(pid) = mark {
                    let name = self.get_name(pid).await;
                    write!(m, "{killer} targets {name}...",);
                } else {
                    write!(m, "{killer} targets nobody...");
                }
                let _ = api::send_group_message(&self.mafia_chat_id, &maf_msg).await;
            }
            Event::Block { blocked, blockers } => {
                let msg = "Your action was blocked...".to_string();
                let _ = api::send_dm(UserId(u64::from(blocked)), &msg).await;
                for blocker in blockers {
                    let msg = "You blocked an action...".to_string();
                    let _ = api::send_dm(UserId(u64::from(blocker)), &msg).await;
                }
            }
            Event::Save { saved, saviors } => {}
            Event::NoKill => {
                write!(m, "Nobody was killed...");
            }
            Event::Kill { killer, mark } => {
                let mark = self.get_name(mark).await;
                write!(m, "{mark} was killed in the Night!");
            }
            Event::Investigate { cop, target, appears_mafia } => {
                let target = self.get_name(target).await;
                let align = if appears_mafia { "Mafia Aligned" } else { "Not Mafia Aligned" };
                let msg = format!("{target} is {}", align);
                let _ = api::send_dm(UserId(u64::from(cop)), &msg).await;
            }
            Event::Milk { milky, target } => {
                let target = self.get_name(target).await;
                write!(m, "{target} received milk");
            }
            Event::End { winner } => {
                write!(m, "{winner} wins!");
                // TODO: end stuff?
                // TODO: reveal roles.
            }
        }
        if !msg.is_empty() {
            let _ = api::send_group_message(&self.main_chat_id, &msg).await;
        }
        Ok(())
    }
}
