use crate::core::{ContractResult, Role, Team};
use crate::discord::*;
use crate::{
    core::{Event, Player},
};

use super::GameChannels;

trait EventHandler {
    fn handle_event(&mut self, event: Event) -> Result<(), DiscordError>;
}

pub struct ResponseEventHandler {
    channels: GameChannels,
    start_players: Vec<Player>,
    _rules: (), // Todo...
}

impl EventHandler for ResponseEventHandler {
    fn handle_event(&mut self, event: Event) -> Result<(), DiscordError> {
        match event {
            Event::Start {
                players, contracts, ..
            } => {
                self.start_players = players.clone();
                let mafia_users = players
                    .iter()
                    .filter(|p| p.role.team() == Team::Mafia)
                    .map(|p| p.user_id)
                    .collect();
                add_users_to_channel(self.channels.mafia, mafia_users)?;
                // Send players role messages
                for player in players {
                    send_to_thread(
                        self.channels.main,
                        player.user_id,
                        format!(
                            "Your Role is {}. You are {}. {}",
                            player.role,
                            player.role.team(),
                            player.role.description(),
                        ),
                    )?;
                }
                for contract in contracts {
                    send_to_thread(
                        self.channels.main,
                        contract.get_holder(),
                        contract.description(),
                    )?;
                }
                // Send main channel start message
                //  (Start_roles info)
                // Send mafia channel start message
            }
            Event::Day { day_no, players } => {
                let thresh = players.len() / 2 + 1;
                send_to_channel(
                    self.channels.main,
                    format!(
                        "Day #{} begins. {} votes required. Elect someone to die!",
                        day_no, thresh,
                    ),
                )?;
                change_channel_permission(self.channels.main, Access::Message)?;
                change_channel_permission(self.channels.mafia, Access::View)?;
            }
            Event::Night { night_no, players } => {
                send_to_channel(self.channels.main, format!("Night #{} falls...", night_no))?;
                change_channel_permission(self.channels.main, Access::View)?;
                change_channel_permission(self.channels.mafia, Access::Message)?;
                let options = players.iter().map(|p| p.user_id).collect();
                for player in &players {
                    let verb = match player.role {
                        Role::COP => "investigate",
                        Role::DOCTOR => "save",
                        Role::STRIPPER => "strip",
                        _ => continue,
                    };
                    send_target_message(self.channels.main, player.user_id, &options, verb)?;
                }
                send_mark_message(self.channels.mafia, &options)?;
            }
            Event::Dawn => {
                send_to_channel(self.channels.main, "Dawn breaks...".to_string())?;
            }
            Event::Vote {
                voter,
                ballot,
                threshold,
                count,
                ..
            } => {
                let votee = match ballot {
                    Some(player) => get_name(player.user_id)?,
                    None => "peace".to_string(),
                };
                send_to_channel(
                    self.channels.main,
                    format!(
                        "{} votes for {}! ({}/{})",
                        get_name(voter.user_id)?,
                        votee,
                        count,
                        threshold,
                    ),
                )?;
            }
            Event::Retract { voter, .. } => {
                send_to_channel(
                    self.channels.main,
                    format!("{} retracts vote", get_name(voter.user_id)?,),
                )?;
            }
            Event::Reveal { celeb } => {
                send_to_channel(
                    self.channels.main,
                    format!("{} is CELEB!", get_name(celeb.user_id)?),
                )?;
            }
            Event::Election { ballot, .. } => {
                let elect = match ballot {
                    Some(player) => {
                        format!("{} has been elected to die...", get_name(player.user_id)?)
                    }
                    None => "No one has been elected to die...".to_string(),
                };

                send_to_channel(self.channels.main, elect)?;
            }
            Event::Target { actor, target } => {
                let target_str = match target {
                    Some(player) => get_name(player.user_id)?,
                    None => "no one".to_string(),
                };
                send_to_thread(
                    self.channels.main,
                    actor.user_id,
                    format!("You have targeted {}", target_str),
                )?;
            }
            Event::Mark { killer, mark } => {
                let mark_str = match mark {
                    Some(player) => get_name(player.user_id)?,
                    None => "no one".to_string(),
                };
                send_to_channel(
                    self.channels.mafia,
                    format!(
                        "{} has marked {} to be killed",
                        get_name(killer.user_id)?,
                        mark_str
                    ),
                )?;
            }
            Event::Strip { stripper, blocked } => {
                send_to_thread(
                    self.channels.main,
                    stripper.user_id,
                    format!("You successfully strip {}!", get_name(blocked.user_id)?),
                )?;
            }
            Event::Block { blocked } => {
                send_to_thread(
                    self.channels.main,
                    blocked.user_id,
                    "You were blocked...".to_string(),
                )?;
            }
            Event::Save { doctor, saved } => {
                send_to_thread(
                    self.channels.main,
                    doctor.user_id,
                    format!("You successfully save {}!", get_name(saved.user_id)?),
                )?;
            }
            Event::Investigate { cop, suspect, role } => {
                send_to_thread(
                    self.channels.main,
                    cop.user_id,
                    format!("{} is {}", get_name(suspect.user_id)?, role.team()),
                )?;
            }
            Event::Kill { mark, .. } => {
                send_to_channel(
                    self.channels.main,
                    format!("{} has been killed!", get_name(mark.user_id)?),
                )?;
            }
            Event::NoKill => {
                send_to_channel(
                    self.channels.main,
                    "Everyone seems to be fine...".to_string(),
                )?;
            }
            Event::Eliminate { player } => {
                send_to_channel(
                    self.channels.main,
                    format!("{} was {}", get_name(player.user_id)?, player.role.team()),
                )?;
            }
            Event::Refocus { new_contract } => {
                send_to_thread(
                    self.channels.main,
                    new_contract.get_holder(),
                    new_contract.description(),
                )?;
            }
            Event::End {
                winner,
                contract_results,
            } => {
                send_to_channel(self.channels.main, format!("{} wins!", winner,))?;
                for result in contract_results {
                    let (user, result) = match result {
                        ContractResult::Success { holder } => (holder, "succeeded!"),
                        ContractResult::Failure { holder } => (holder, "failed!"),
                    };
                    send_to_channel(self.channels.main, format!("{} {}", user, result))?;
                }
            }
            _ => todo!(),
        }
        Ok(())
    }
}
