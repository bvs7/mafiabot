use std::collections::HashSet;

use mafia::state::{status, EventHandler};
use tokio::sync::TryLockError;
use tokio::task::{block_in_place, JoinSet};

use crate::app::AppState;
use crate::prelude::*;

// What do we need here?

#[derive(Debug, Clone)]
pub struct GroupMeEventHandler {
    game_id: GameId,
    main_id: GroupId,
    mafia_id: GroupId,
    app_state: Arc<AppState>,
}

impl GroupMeEventHandler {
    pub fn new(
        game_id: GameId,
        main_id: GroupId,
        mafia_id: GroupId,
        app_state: Arc<AppState>,
    ) -> Self {
        Self { game_id, main_id, mafia_id, app_state }
    }
}

impl GroupMeEventHandler {
    fn name(&self, pid: Pid) -> String {
        for _ in 0..100 {
            match self.app_state.groups.try_read() {
                Ok(r_groups) => {
                    let Some(group) = r_groups.get(&self.main_id) else {
                        error!("Group {} not found", self.main_id);
                        continue;
                    };
                    let Some(name) = group.name(u64::from(pid)) else {
                        warn!("User {} not found in group {}", pid, self.main_id);
                        continue;
                    };
                    return name.to_string();
                }
                Err(TryLockError) => {}
            }
        }
        error!("Failed 100 times to get name for {}", pid);
        format!("User: {}", pid)
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

fn option_msg(options: Vec<Pid>, names: &HashMap<Pid, String>) -> String {
    let mut msg = "Choose a target:\n".to_string();
    for pid in options {
        let name = names.get(&pid).unwrap();
        msg.push_str(format!("/vote {} {}\n", pid, name).as_str());
    }
    msg
}

impl EventHandler for GroupMeEventHandler {
    #[tracing::intstrument]
    fn handle(&mut self, event: Event) {
        let js = JoinSet::new();
        match event {
            Event::Start { players, rules } => {
                for (pid, role) in players.iter() {
                    let user_id = UserId(u64::from(*pid));
                    let msg = create_start_msg(*role, &names);
                    api::send_group_message(&self.main_id, &msg);
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

                let players = self.status_rx.borrow().players.clone();
                let names = self.status_rx.borrow().names.clone();
                let opt_msg = option_msg(players, &names);
                let _ = api::send_group_message(&self.mafia_id, &opt_msg).await;
                for targeter in self.targeters.iter() {
                    let _ = api::send_dm(UserId(u64::from(*targeter)), &opt_msg).await;
                }
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
            // TODO: add thresh to ballot and former?
            Event::Vote { voter, ballot, former } => {
                let n = self.status_rx.borrow().players.len();
                let thresh = n / 2 + 1;
                let pthresh = (n + 1) / 2;
                let mut msg = "".to_string();
                let voter = self.name(voter);
                if let Some((choice, count)) = ballot {
                    if let Some(pid) = choice {
                        let name = self.name(pid);
                        msg.push_str(
                            format!("{voter} votes for {name} ({count}/{thresh})").as_str(),
                        );
                    } else {
                        msg.push_str(
                            format!("{voter} votes for peace.({count}/{pthresh})").as_str(),
                        );
                    }
                } else {
                    msg.push_str(format!("{voter} retracts their vote.").as_str());
                }
                if let Some((choice, count)) = former {
                    if let Some(pid) = choice {
                        let name = self.name(pid);
                        msg.push_str(format!("\n({name} still has {count}/{thresh})").as_str());
                    } else {
                        msg.push_str(format!("\n(peace still has {count}/{pthresh})").as_str());
                    }
                }
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Reveal { celeb } => {
                let msg = format!("{celeb} reveals, they are CELEB!\n",);
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Election { choice, hammer, voters } => {
                if let Some(pid) = choice {
                    let name = self.name(pid);
                    let msg = format!("{name} is elected!",);
                    let _ = api::send_group_message(&self.main_id, &msg).await;
                } else {
                    let msg = "Nobody has been elected.".to_string();
                    let _ = api::send_group_message(&self.main_id, &msg).await;
                }
            }
            Event::Dawn => {
                let msg = "Dawn breaks...".to_string();
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Eliminate { player, role, context } => {
                self.targeters.remove(&player);
                let name = self.name(player);
                let team = role.team();
                let msg = format!("{name} was {team}!",);
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Target { actor, choice } => {
                let mut msg = "".to_string();
                if let Some(pid) = choice {
                    let name = self.name(pid);
                    msg.push_str(format!("You target {name}...",).as_str());
                } else {
                    msg.push_str("You target nobody...");
                }
                let _ = api::send_dm(UserId(u64::from(actor)), &msg).await;
            }
            Event::Scheme { killer, mark } => {
                let mut msg = "".to_string();
                let actor = self.name(killer);
                if let Some(pid) = mark {
                    let name = self.name(pid);
                    msg.push_str(format!("{killer} targets {name}...",).as_str());
                } else {
                    msg.push_str(format!("{killer} targets nobody...").as_str());
                }
                let _ = api::send_group_message(&self.mafia_id, &msg).await;
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
                let msg = "Nobody was killed...".to_string();
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Kill { killer, mark } => {
                let mark = self.name(mark);
                let msg = format!("{mark} was killed in the Night!",);
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::Investigate { cop, target, appears_mafia } => {
                let target = self.name(target);
                let align = if appears_mafia { "Mafia Aligned" } else { "Not Mafia Aligned" };
                let msg = format!("{target} is {}", align);
                let _ = api::send_dm(UserId(u64::from(cop)), &msg).await;
            }
            Event::Milk { milky, target } => {
                let target = self.name(target);
                let msg = format!("{target} received milk",);
                let _ = api::send_group_message(&self.main_id, &msg).await;
            }
            Event::End { winner } => {
                let msg = format!("{winner} wins!",);
                let _ = api::send_group_message(&self.main_id, &msg).await;
                // TODO: end stuff?
                // TODO: reveal roles.
            }
        }
    }
}
