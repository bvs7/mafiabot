use std::collections::{HashMap, HashSet};

use axum::Json;
use reqwest::Client;
use serde_json::Value as JsonValue;

use crate::engine::{
    interface::{Action, Event},
    state::{
        id::{Gid, Pid},
        role::{Role, Team},
        rules::Rules,
    },
    Game,
};

use super::{api, util::json_access};

/// The wrapper that holds a game and the relevant context needed for it

struct GameContext {
    game: Game,
    game_id: Gid,
    main_chat_id: String,
    mafia_chat_id: String,
    names: HashMap<Pid, String>,
    registry: Vec<(Pid, Role)>, // For sending options to users
}

impl GameContext {
    // Think, how is new used for this?
    // What are inputs? A new game needs registry and rules
    // We should probably start the event handler, wait a moment, then start the game?
    async fn new(
        users: Vec<u64>,
        rules: Rules,
        main_chat_id: String,
        mafia_chat_id: String,
    ) -> Self {
        // Do we get roles from rolegen here? Hmmm maybe...
        let roles: Vec<Role> = todo!();
        let registry = users.iter().copied().zip(roles).collect();
        let game = Game::new(registry, rules);
        let game_id = todo!(); //game.game_id().await;

        // Add members to main chat
        let client = Client::new();
        // Get names
        let mut members = Vec::new();
        let group = api::get_group(&client, &super::LOBBY_CHAT_ID.to_string())
            .await
            .unwrap();
        let seen_members: Vec<JsonValue> = json_access(&group, "response.members").unwrap();
        for member in seen_members {
            let user_id: u64 = json_access(&member, "user_id").unwrap();
            let nickname: String = json_access(&member, "nickname").unwrap();
            members.push((nickname, user_id));
        }
        api::add_members(&client, &main_chat_id, members.clone())
            .await
            .unwrap();
        let mafia: HashSet<_> = registry
            .iter()
            .filter(|(_, role)| role.team() == Team::Mafia)
            .map(|(pid, _)| *pid)
            .collect();
        members.retain(|(_, id)| mafia.contains(id));
        api::add_members(&client, &mafia_chat_id, members)
            .await
            .unwrap();
        Self {
            game,
            game_id,
            main_chat_id,
            mafia_chat_id,
            names: HashMap::new(),
            registry: registry.into_iter().map(|(p, r)| (p.into(), r)).collect(),
        };
    }

    pub async fn start(self) {
        self.game.start_action_handler().unwrap();
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
        self.game
            .action_tx()
            .send((Action::Start, resp_tx))
            .await
            .unwrap();
        tokio::spawn(self.event_handler());
    }

    fn name_of(&self, pid: &Pid) -> String {
        let unknown = "_".to_string();
        self.names.get(pid).unwrap_or(&unknown).clone()
    }

    async fn event_handler(mut self) {
        let client = Client::new();
        let mut event_rx = self.game.event_rx().await;
        while let Ok(event) = event_rx.recv().await {
            let ctx = &mut self;
            match event {
                Event::Start {
                    // id,
                    players,
                    rules,
                    // counts,
                } => {
                    ctx.registry = players.clone();
                    // TODO: Update names
                    // Send roles
                    for (pid, role) in players.iter() {
                        let msg = format!(
                            "Game: {}\nRole: {}\n(Team: {})",
                            ctx.game_id,
                            role.kind(),
                            role.team()
                        );
                        api::send_dm(&client, (*pid).into(), &msg).await.unwrap();
                    }
                    // Send start message
                    let mut msg = String::new();
                    msg.push_str(&format!("Game #{} starting!", ctx.game_id));
                    msg.push_str(&format!("\n{} Players:", players.len()));
                    for (pid, _) in &ctx.registry {
                        let name = ctx.name_of(pid);
                        msg.push_str(&format!("\n  {}", name));
                    }
                    // let mut counts_str = String::from("\nTeams:");
                    // if let Some(town) = counts.get(&Team::Town) {
                    //     counts_str.push_str(&format!("\n  Town: {}", town));
                    // }
                    // if let Some(mafia) = counts.get(&Team::Mafia) {
                    //     counts_str.push_str(&format!("\n  Mafia: {}", mafia));
                    // }
                    // if let Some(rogue) = counts.get(&Team::Rogue) {
                    //     counts_str.push_str(&format!("\n  Rogue: {}", rogue));
                    // }
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Day { day, counts } => {
                    let mut msg = String::new();
                    msg.push_str(&format!("Day {} begins", day));
                    let mut counts_str = String::from("\nTeams:");
                    if let Some(town) = counts.get(&Team::Town) {
                        counts_str.push_str(&format!("\n  Town: {}", town));
                    }
                    if let Some(mafia) = counts.get(&Team::Mafia) {
                        counts_str.push_str(&format!("\n  Mafia: {}", mafia));
                    }
                    if let Some(rogue) = counts.get(&Team::Rogue) {
                        counts_str.push_str(&format!("\n  Rogue: {}", rogue));
                    }
                    let n = ctx.registry.len();
                    let thresh = (n / 2) + 1;
                    msg.push_str(&format!(
                        "\n{} players remain. {} votes needed to elect!",
                        n, thresh
                    ));
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Night { day, counts } => {
                    let mut msg = String::new();
                    msg.push_str(&format!("Night {} begins", day));
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                    // Send options
                    let mut opt_msg = "Targets:".to_string();
                    for (i, (pid, _)) in ctx.registry.iter().enumerate() {
                        let name = ctx.name_of(pid);
                        opt_msg.push_str(&format!("\n  {}: {}", i, name));
                    }
                    for (pid, role) in &ctx.registry {
                        if role.is_targeting() {
                            api::send_dm(&client, (*pid).into(), &opt_msg)
                                .await
                                .unwrap();
                        }
                    }
                    api::send_group_message(&client, &ctx.mafia_chat_id, &opt_msg)
                        .await
                        .unwrap();
                }
                Event::Vote {
                    voter,
                    ballot,
                    former,
                } => {
                    let n = ctx.registry.len();
                    let thresh = (n / 2) + 1;
                    let pthresh = (n + 1) / 2;
                    let mut msg = String::new();
                    if let Some((choice, count)) = ballot {
                        if let Some(pid) = choice {
                            let name = ctx.name_of(&pid);
                            msg.push_str(&format!("{}/{} votes for {}", count, thresh, name));
                        } else {
                            msg.push_str(&format!("{}/{} votes for peace", count, pthresh));
                        }
                    } else {
                        // Vote was a retraction
                        msg.push_str("Vote retracted");
                    }
                    if let Some((choice, count)) = former {
                        if let Some(pid) = choice {
                            let name = ctx.name_of(&pid);
                            msg.push_str(&format!("\n (still {}/{} for {})", count, thresh, name));
                        } else {
                            msg.push_str(&format!("\n (still {}/{} for peace)", count, pthresh));
                        }
                    }
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Reveal { celeb } => {
                    let name = ctx.name_of(&celeb);
                    let msg = format!("{} is CELEB!", name);
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Election {
                    choice,
                    hammer,
                    voters,
                } => {
                    let n = ctx.registry.len();
                    let thresh = (n / 2) + 1;
                    let mut msg = String::new();
                    if let Some(pid) = choice {
                        let name = ctx.name_of(&pid);
                        msg.push_str(&format!("{} is elected to die", name));
                    } else {
                        msg.push_str("Nobody is elected to die");
                    }
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Dawn => {
                    let msg = "Dawn breaks";
                    api::send_group_message(&client, &ctx.main_chat_id, msg)
                        .await
                        .unwrap();
                }
                Event::Eliminate {
                    player,
                    role,
                    context,
                } => {
                    let name = ctx.name_of(&player);
                    let msg = format!("{} was {}", name, Team::from(role));
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                    ctx.registry.retain(|&(pid, _)| pid != player);
                }
                Event::Target { actor, choice } => {
                    let name = choice
                        .map(|pid| ctx.name_of(&pid))
                        .unwrap_or("nobody".to_string());
                    let msg = format!("You target {}", name);
                    api::send_dm(&client, actor.into(), &msg).await.unwrap();
                }
                Event::Scheme { killer, mark } => {
                    let killer_str = ctx.name_of(&killer);
                    let mark_str = mark
                        .map(|pid| ctx.name_of(&pid))
                        .unwrap_or("nobody".to_string());
                    let msg = format!("{} prepares to kill {}", killer_str, mark_str);
                    api::send_group_message(&client, &ctx.mafia_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Block { blocked, blockers } => {
                    let msg = "Your action was blocked";
                    api::send_dm(&client, blocked.into(), msg).await.unwrap();
                    // let msg = "You blocked an action";
                    // for pid in blockers {
                    //     api::send_dm(&client, pid.into(), msg).await.unwrap();
                    // }
                }
                Event::Save { saved, saviors } => {}
                Event::Kill { killer, mark } => {
                    let mark_str = ctx.name_of(&mark);
                    let msg = format!("{} was killed!", mark_str);
                    api::send_group_message(&client, &ctx.main_chat_id, &msg)
                        .await
                        .unwrap();
                }
                Event::Investigate {
                    cop,
                    target,
                    appears_mafia,
                } => {
                    let name = ctx.name_of(&target);
                    let mut msg = format!("{} seems ", name);
                    msg.push_str(if appears_mafia {
                        "Mafia Aligned"
                    } else {
                        "Not Mafia Aligned"
                    });
                    api::send_dm(&client, cop.into(), &msg).await.unwrap();
                }
                Event::End { winner } => {
                    let msg = if winner == Team::Town {
                        "Town wins!"
                    } else if winner == Team::Mafia {
                        "Mafia wins!"
                    } else {
                        "Rogue wins?..."
                    };
                    api::send_group_message(&client, &ctx.main_chat_id, msg)
                        .await
                        .unwrap();
                    // TODO: get Player log from the start list
                }
                _ => unimplemented!(),
            }
        }
    }
}

mod sync {
    use crate::engine::{
        interface::Event,
        state::{id::Pid, role::Role, rules::Rules},
    };
    use std::{
        io::Cursor,
        sync::{Arc, Mutex},
    };
}
