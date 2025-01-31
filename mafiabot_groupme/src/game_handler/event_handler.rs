use crate::prelude::*;

use std::fmt::Write;
use tokio::sync::broadcast::error::RecvError;

// Needs to know players/roles, right?
#[derive(Debug)]
pub struct EventHandler {
    game_id: GameId,
    event_rx: EventRx,
    lobby_id: GroupId,
    main_chat_id: GroupId,
    mafia_chat_id: GroupId,
    players: HashMap<Pid, Role>, // Cached players TODO have status just have roles???
    names: HashMap<UserId, String>,
    app_state: Arc<AppState>,
}
impl EventHandler {
    pub fn new(
        game_id: GameId,
        event_rx: EventRx,
        lobby_id: GroupId,
        main_chat_id: GroupId,
        mafia_chat_id: GroupId,
        app_state: Arc<AppState>,
    ) -> Self {
        Self {
            game_id,
            lobby_id,
            main_chat_id,
            mafia_chat_id,
            event_rx,
            players: HashMap::new(),
            names: HashMap::new(),
            app_state,
        }
    }

    #[tracing::instrument]
    pub async fn run(mut self) {
        loop {
            match self.event_rx.recv().await {
                Some(event) => match self.handle_event(event).await {
                    Ok(_) => continue,
                    Err(e) => error!("Error handling event: {:?}", e),
                },
                None => {
                    info!("Got Closed, closing");
                    break;
                }
            }
        }
    }

    // TODO: every once in a while, update names anyways
    async fn get_name(&mut self, pid: Pid) -> Result<String, std::fmt::Error> {
        let user_id = W(pid).into();
        for _ in 0..3 {
            if let Some(name) = self.names.get(&user_id) {
                return Ok(name.clone());
            } else {
                self.app_state.update_names(&self.main_chat_id).await;
                let new_names = self.app_state.get_names(&self.main_chat_id).await;
                self.names.extend(new_names.into_iter());
            }
        }
        warn!("Could not get name for pid: {}", pid);
        Ok(format!("(Player ID {pid})"))
    }

    async fn create_start_msg(&mut self, role: Role) -> Result<String, std::fmt::Error> {
        let mut msg = String::new();
        let m = &mut msg;
        write!(m, "Your Role is {}, ", role)?;
        write!(m, "you are {} aligned.\n", role.team())?;
        write!(m, "Use /help {} or /help {} for more info", role, role.team())?;
        match role {
            Role::GUARD(charge) | Role::AGENT(charge) => {
                write!(m, "Your charge is {}", self.get_name(charge).await?)?;
            }
            _ => {}
        }
        Ok(msg)
    }

    async fn option_msg(&mut self) -> Result<String, std::fmt::Error> {
        let mut msg = String::new();
        let m = &mut msg;
        write!(m, "Choose a target:\n")?;
        let mut c = 'A';
        let players: Vec<_> = self.players.iter().map(|(pid, _)| *pid).collect();
        for pid in players {
            let name = self.get_name(pid).await?;
            write!(m, "{}: {}\n", c, name)?;
            c = (c as u8 + 1) as char;
        }
        Ok(msg)
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<(), std::fmt::Error> {
        let mut msg = String::new();
        let m = &mut msg;
        let mut js = tokio::task::JoinSet::new();
        // TODO: every once in a while, just update players to be sure?
        match event {
            Event::Start { players, rules } => {
                write!(m, "Game {} begins!\nPlayers:", self.game_id)?;
                for (pid, _) in players.iter() {
                    let name = self.get_name(*pid).await?;
                    write!(m, "\n  {}", name)?;
                }
                for (pid, role) in players.iter() {
                    js.spawn({
                        let user_id: UserId = W(*pid).into();
                        let start_msg = self.create_start_msg(*role).await?;
                        async move {
                            let _ = api::send_dm(user_id, &start_msg).await;
                        }
                    });
                }
                let mafia_msg = format!("Welcome to the Mafia Chat for game {}!", self.game_id);
                js.spawn({
                    let mafia_chat_id = self.mafia_chat_id.clone();
                    let mafia_msg = mafia_msg.clone();
                    async move {
                        let _ = api::send_group_message(&mafia_chat_id, &mafia_msg).await;
                    }
                });
            }
            Event::Day { day, counts } => {
                write!(m, "Day {} proceeds...\n", day)?;
                if let Some(count) = counts.get(&Team::Town) {
                    write!(m, " Town: {}\n", count)?;
                }
                if let Some(count) = counts.get(&Team::Mafia) {
                    write!(m, " Mafia: {}\n", count)?;
                }
                if let Some(count) = counts.get(&Team::Rogue) {
                    write!(m, " Rogue: {}\n", count)?;
                }
            }
            Event::Night { day, counts } => {
                write!(m, "Night {} falls...\n", day)?;
                let opt = self.option_msg().await?;
                for (pid, role) in self.players.iter() {
                    if role.is_targeting() {
                        js.spawn({
                            let pid = pid.clone();
                            let opt = opt.clone();
                            async move {
                                let _ = api::send_dm(W(pid).into(), &opt).await;
                            }
                        });
                    }
                }
            }
            Event::Eclipse { avenger, hammer, guilty } => {
                let avenger = self.get_name(avenger).await?;
                write!(
                    m,
                    "The sky darkens as the moon eclipses the sun... {avenger} \
                    will /vote one of their voters to die!\n"
                )?;
            }
            Event::Vengeance { avenger, victim } => {
                let avenger = self.get_name(avenger).await?;
                let victim = self.get_name(victim).await?;
                write!(m, "{avenger} has chosen {victim} to die with them!\n")?;
            }
            // TODO: add thresh to ballot and former?
            Event::Vote { voter, ballot, former } => {
                let n = self.players.len();
                let thresh = n / 2 + 1;
                let pthresh = (n + 1) / 2;
                let voter = self.get_name(voter).await?;
                if let Some((choice, count)) = ballot {
                    if let Some(pid) = choice {
                        let name = self.get_name(pid).await?;
                        write!(m, "{voter} votes for {name} ({count}/{thresh})")?;
                    } else {
                        write!(m, "{voter} votes for peace.({count}/{pthresh})")?;
                    }
                } else {
                    write!(m, "{voter} retracts their vote.")?;
                }
                if let Some((choice, count)) = former {
                    if let Some(pid) = choice {
                        let name = self.get_name(pid).await?;
                        write!(m, "\n({name} still has {count}/{thresh})")?;
                    } else {
                        write!(m, "\n(peace still has {count}/{pthresh})")?;
                    }
                }
            }
            Event::Reveal { celeb } => {
                write!(m, "{celeb} reveals, they are CELEB!\n")?;
            }
            Event::Election { choice, hammer, voters } => {
                if let Some(pid) = choice {
                    let name = self.get_name(pid).await?;
                    write!(m, "{name} is elected!")?;
                } else {
                    write!(m, "Nobody has been elected.")?;
                }
            }
            Event::Dawn => {
                write!(m, "Dawn breaks...")?;
            }
            // TODO: reveal roles to the dead?
            Event::Eliminate { player, role, context } => {
                let name = self.get_name(player).await?;
                self.players.remove(&player);
                let team = role.team();
                write!(m, "{name} was {team}!")?;
            }
            Event::Target { actor, choice } => {
                let mut dm = String::new();
                let m = &mut dm;
                if let Some(pid) = choice {
                    let name = self.get_name(pid).await?;
                    write!(m, "You target {name}...")?;
                } else {
                    write!(m, "You target nobody...")?;
                }
                js.spawn({
                    let actor = actor.clone();
                    let dm = dm.clone();
                    async move {
                        let _ = api::send_dm(W(actor).into(), &dm).await;
                    }
                });
            }
            Event::Scheme { killer, mark } => {
                let mut maf_msg = "".to_string();
                let m = &mut maf_msg;
                let actor = self.get_name(killer).await?;
                if let Some(pid) = mark {
                    let name = self.get_name(pid).await?;
                    write!(m, "{killer} targets {name}...")?;
                } else {
                    write!(m, "{killer} targets nobody...")?;
                }
                js.spawn({
                    let maf_chat_id = self.mafia_chat_id.clone();
                    let maf_msg = maf_msg.clone();
                    async move {
                        let _ = api::send_group_message(&maf_chat_id, &maf_msg).await;
                    }
                });
            }
            Event::Block { blocked, blockers } => {
                let msg = "Your action was blocked...".to_string();
                js.spawn({
                    let blocked = W(blocked).into();
                    let msg = msg.clone();
                    async move {
                        let _ = api::send_dm(blocked, &msg).await;
                    }
                });
                for blocker in blockers {
                    let msg = "You blocked an action...".to_string();
                    js.spawn({
                        let blocker = W(blocker).into();
                        let msg = msg.clone();
                        async move {
                            let _ = api::send_dm(blocker, &msg).await;
                        }
                    });
                }
            }
            Event::Save { saved, saviors } => {}
            Event::NoKill => {
                write!(m, "Nobody was killed...")?;
            }
            Event::Kill { killer, mark } => {
                let mark = self.get_name(mark).await?;
                write!(m, "{mark} was killed in the Night!")?;
            }
            Event::Investigate { cop, target, appears_mafia } => {
                let target = self.get_name(target).await?;
                let align = if appears_mafia { "Mafia Aligned" } else { "Not Mafia Aligned" };
                let msg = format!("{target} is {}", align);
                js.spawn({
                    let cop = W(cop).into();
                    let msg = msg.clone();
                    async move {
                        let _ = api::send_dm(cop, &msg).await;
                    }
                });
            }
            Event::Milk { milky, target } => {
                let target = self.get_name(target).await?;
                write!(m, "{target} received milk")?;
            }
            Event::End { winner } => {
                write!(m, "{winner} wins!")?;
                // TODO: end stuff?
                // TODO: reveal roles.
            }
        }
        if !msg.is_empty() {
            js.spawn({
                let main_chat_id = self.main_chat_id.clone();
                let msg = msg.clone();
                async move {
                    let _ = api::send_group_message(&main_chat_id, &msg).await;
                }
            });
        }
        let _ = js.join_all().await;
        Ok(())
    }
}
