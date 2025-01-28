use api::Member;
use mafia::{
    game::{self, EventHandler},
    state::status,
};

use crate::prelude::*;

use async_trait::async_trait;

pub struct GroupMeEventHandler {
    game_id: GameId,
    lobby_id: GroupId,
    main_id: GroupId,
    mafia_id: GroupId,
    status_rx: watch::Receiver<Status>,
    app_status: Arc<RwLock<AppStatus>>,
}

impl GroupMeEventHandler {
    pub async fn new(game: &Game, lobby_id: GroupId, app_status: Arc<RwLock<AppStatus>>) -> Self {
        let game_id = game.id();
        let r_app_status = app_status.read().await;
        let game_info = r_app_status.games.get(&game_id).expect("Game not found");
        let status_rx = game_info.status.clone();
        let main_id = game_info.main_id.clone();
        let mafia_id = game_info.mafia_id.clone();
        drop(r_app_status);
        Self { game_id, lobby_id, main_id, mafia_id, status_rx, app_status }
    }

    fn name(&self, pid: Pid) -> String {
        let status = self.status_rx.borrow();
        status.names.get(&pid).cloned().unwrap_or_else(|| format!("Player {}", pid))
    }
}

fn create_start_msg(role: Role, names: &HashMap<Pid, String>) -> String {
    let mut msg = format!("Your Role is {}, ", role);
    msg.push_str(format!("you are {} aligned.\n", role.team()).as_str());
    msg.push_str(format!("Use /help {} or /help {} for more info", role, role.team()).as_str());
    match role {
        Role::GUARD(charge) | Role::AGENT(charge) => {
            msg.push_str(format!("Your charge is {}", names.get(&charge).unwrap()).as_str());
        }
        _ => {}
    }
    msg
}

#[async_trait]
impl EventHandler for GroupMeEventHandler {
    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start { players, rules } => {
                // Add players to chats, send start messages
                let r_app_status = self.app_status.read().await;
                let lobby = r_app_status.lobbies.get(&self.lobby_id).expect("Lobby not found");
                let names = lobby.chat.names.clone();
                drop(r_app_status);
                let mut new_main_members = Vec::new();
                let mut new_mafia_members = Vec::new();
                for (pid, role) in players.iter() {
                    let pid = *pid;
                    let user_id = UserId(u64::from(pid));
                    // Send start message

                    let nickname =
                        names.get(&user_id).cloned().unwrap_or_else(|| format!("Player {}", pid));
                    let member: Member = (nickname, user_id).into();
                    if role.is_mafia() {
                        new_mafia_members.push(member.clone());
                    }
                    new_main_members.push(member);
                }
                // Add members to chats
                let mut w_app_status = self.app_status.write().await;

                let main_chat =
                    w_app_status.groups.get_mut(&self.main_id).expect("Main chat not found");
                main_chat.add_members(new_main_members).await;
                let names = main_chat.names.clone();

                let mafia_chat =
                    w_app_status.groups.get_mut(&self.mafia_id).expect("Mafia chat not found");
                mafia_chat.add_members(new_mafia_members).await;

                drop(w_app_status);

                // Send start messages
                let names = names.into_iter().map(|(u, n)| (Pid::from(u.0), n)).collect();
                for (pid, role) in players.iter() {
                    let user_id = UserId(u64::from(*pid));
                    let msg = create_start_msg(*role, &names);
                    let _ = api::send_group_message(&self.main_id, &msg).await;
                }

                // Send group chat messages
                let mut main_msg = format!("Game {} begins!\nPlayers:", self.game_id);
                for (pid, _) in players.iter() {
                    let name = self.name(*pid);
                    main_msg.push_str(format!("\n  {}", name).as_str());
                }
                let _ = api::send_group_message(&self.main_id, &main_msg).await;
                let mafia_msg = format!("Welcome to the Mafia Chat for game {}!", self.game_id);
                let _ = api::send_group_message(&self.mafia_id, &mafia_msg).await;
            }
            Event::Day { day, counts } => {
                let mut msg = format!("Day {} proceeds...\n", day);
                if let Some(count) = counts.get(&Team::Town) {
                    msg.push_str(format!(" Town: {}\n", count).as_str());
                }
                if let Some(count) = counts.get(&Team::Mafia) {
                    msg.push_str(format!(" Mafia: {}\n", count).as_str());
                }
                if let Some(count) = counts.get(&Team::Rogue) {
                    msg.push_str(format!(" Rogue: {}\n", count).as_str());
                }
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Night { day, counts } => {
                let msg = format!("Night {} falls...\n", day);
                let _ = api::send_group_message(&self.main_id, &msg).await;
                // TODO: send options
            }
            Event::Eclipse { avenger, hammer, guilty } => {
                let avenger = self.name(avenger);
                let msg = format!(
                    "The sky darkens as the moon eclipses the sun... {avenger} \
                will /vote for one of those who voted, to follow them into the end!\n"
                );
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Vengeance { avenger, victim } => {
                let avenger = self.name(avenger);
                let victim = self.name(victim);
                let msg = format!("{avenger} has chosen {victim} to die with them!\n",);
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Vote { voter, ballot, former } => {
                let mut msg = "".to_string();
                let voter = self.name(voter);
                if let Some((choice, count)) = ballot {
                    if let Some(pid) = choice {
                        let name = self.name(pid);
                        msg.push_str(format!("{voter} votes for {name} \n",).as_str());
                    } else {
                        msg.push_str(format!("{voter} votes for peace.\n",).as_str());
                    }
                }
            }
            Event::Reveal { celeb } => {
                todo!()
            }
            Event::Election { choice, hammer, voters } => {
                todo!()
            }
            Event::Dawn => {
                todo!()
            }
            Event::Eliminate { player, role, context } => {
                todo!()
            }
            Event::Target { actor, choice } => {
                todo!()
            }
            Event::Scheme { killer, mark } => {
                todo!()
            }
            Event::Block { blocked, blockers } => {
                todo!()
            }
            Event::Save { saved, saviors } => {
                todo!()
            }
            Event::NoKill => {
                todo!()
            }
            Event::Kill { killer, mark } => {
                todo!()
            }
            Event::Investigate { cop, target, appears_mafia } => {
                todo!()
            }
            Event::Milk { milky, target } => {
                todo!()
            }
            Event::End { winner } => {
                todo!()
            }
        }
    }
}
