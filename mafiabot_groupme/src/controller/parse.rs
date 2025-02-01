use crate::prelude::*;

impl ControllerHandle {
    async fn perform_cmd(&mut self, cmd: Command, resp: RespContext) {
        match cmd {
            Command::Lobby(group_id, cmd) => {
                let lobbies = self.lobbies.borrow();
                let lobby = lobbies.get(&group_id).expect("Lobby should exist");
                lobby.perform_lobby_cmd(cmd, resp).await;
            }
            Command::Game(game_id, cmd) => {
                let games = self.games.borrow();
                let game = games.get(&game_id).expect("Game should exist");
                game.perform_game_cmd(cmd, resp).await;
            }
            Command::App(user_id, cmd) => match cmd {
                AppCommand::GetFocus => {
                    todo!()
                }
                AppCommand::Focus { game_id } => {
                    todo!()
                }
            },
        }
    }

    async fn parse_cmd(&self, data: Data) -> Option<(Parse<Command>, RespContext)> {
        let text = data.text();
        let mut chars = text.chars();
        let Some('/') = chars.next() else {
            return None;
        };
        let text: String = chars.collect();
        match data {
            Data::GroupMsg { group_id, user_id, attachments, id, .. } => {
                if let Some(cmd) = self.parse_group_cmd(&group_id, user_id, text, attachments).await
                {
                    return Some((cmd, RespContext::Group(group_id, id)));
                }
            }
            Data::DirectMsg { attachments, name, text, user_id, id, .. } => {
                todo!()
            }
            Data::Unknown => {
                todo!()
            }
        }
        None
    }

    async fn parse_group_cmd(
        &self,
        group_id: &GroupId,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Option<Parse<Command>> {
        // Check for a lobby
        let lobbies = self.lobbies.borrow();
        let games = self.games.borrow();
        let cmd = if let Some(lobby) = lobbies.get(group_id) {
            self.parse_lobby_cmd(lobby, user_id, text, attachments)
                .await
                .map(|p| p.map(|cmd| Command::Lobby(*group_id, cmd)))
        } else if let Some((game_id, game)) =
            games.iter().find(|(g_id, g)| &g.main_chat_id == group_id)
        {
            self.parse_main_chat_cmd(user_id, &text, attachments)
                .await
                .map(|p| p.map(|cmd| Command::Game(*game_id, cmd)))
        } else if let Some((game_id, game)) =
            games.iter().find(|(g_id, g)| &g.mafia_chat_id == group_id)
        {
            self.parse_mafia_chat_cmd(game, user_id, &text)
                .await
                .map(|p| p.map(|cmd| Command::Game(*game_id, cmd)))
        } else {
            None
        };
        todo!("Parse an app command")
    }

    async fn parse_lobby_cmd(
        &self,
        lobby: &LobbyHandle,
        user_id: UserId,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Option<Parse<LobbyCommand>> {
        use LobbyCommand::*;
        let user_id = W(user_id);
        let mut words = text.split_whitespace();
        let cmd = match (words.next(), words.next(), words.next()) {
            (Some("start"), mins, min_ps) => {
                let mut minutes = 10;
                if let Some(Ok(m)) = mins.map(|s| s.parse::<u64>()) {
                    if minutes <= 120 {
                        minutes = m;
                    }
                }
                let mut min_players = 5_usize;
                if let Some(Ok(m)) = min_ps.map(|s| s.parse::<usize>()) {
                    min_players = m;
                }
                Start { minutes, min_players }
            }
            (Some("status"), Some(game_id), _) => {
                let Some(game_id) = game_id.parse::<u64>().ok().map(GameId::from) else {
                    return Some(Err(format!("Could not parse game id: {}", game_id)));
                };
                StatusOf { game_id }
            }
            (Some("status"), None, _) => Status,
            _ => return None,
        };
        Some(Ok(cmd))
    }

    async fn parse_main_chat_cmd(
        &self,
        user_id: UserId,
        text: &str,
        attachments: Vec<Attachment>,
    ) -> Option<Parse<GameCommand>> {
        let words = text.split_whitespace().collect::<Vec<&str>>();
        let mentions = attachments.into_iter().find(|a| matches!(a, Attachment::Mentions { .. }));
        use Attachment::Mentions;
        use GameCommand::*;
        let user_id = W(user_id);
        let cmd = match words[..] {
            ["vote", "nokill", ..] | ["vote", "none", ..] => Vote { user_id, ballot: Some(None) },
            ["unvote", ..] => Vote { user_id, ballot: None },
            ["vote", ..] => {
                if let Some(Mentions { user_ids }) = mentions {
                    let Some(other) = user_ids.first() else {
                        return Some(Err(format!("Could not parse mentions")));
                    };
                    let ballot = Some(Some(W(UserId(*other))));
                    Vote { user_id, ballot }
                } else {
                    Vote { user_id, ballot: None }
                }
            }
            ["status", ..] => Status,
            _ => return None,
        };
        Some(Ok(cmd))
    }

    async fn parse_mafia_chat_cmd(
        &self,
        game: &GameHandle,
        user_id: UserId,
        text: &str,
    ) -> Option<Parse<GameCommand>> {
        let words = text.split_whitespace().collect::<Vec<&str>>();
        use GameCommand::*;
        let user_id = W(user_id);
        let cmd = match words[..] {
            ["target", target, ..] => {
                // match implies target is not empty
                let bytes = target.as_bytes();
                let [c] = bytes[..] else {
                    return Some(Err(format!("Could not parse target {}", target)));
                };
                let Ok(target_id) = game.get_target(c).await else {
                    return Some(Err(format!("Could not find target {}", target)));
                };
                let target = target_id.map(|t| W(UserId(t)));
                Scheme { user_id, target }
            }
            _ => return None,
        };
        Some(Ok(cmd))
    }

    async fn parse_dm_cmd(&self, user_id: UserId, text: String) -> Option<Parse<Command>> {
        let focus = self.focus.borrow();
        let focus = focus.get(&user_id).copied();
        let words = text.split_whitespace().collect::<Vec<&str>>();
        let cmd = match words[..] {
            ["focus", game_id, ..] => {
                let Some(game_id) = game_id.parse::<u64>().ok().map(GameId::from) else {
                    return Some(Err(format!("Could not parse game id: {}", game_id)));
                };
                Command::App(user_id, AppCommand::Focus { game_id })
            }
            ["focus"] => Command::App(user_id, AppCommand::GetFocus),
            ["target", target, ..] => {
                let user_id = W(user_id);
                let Some(game_id) = focus else {
                    return Some(Err(
                        "You have no focused game in which you can target".to_string()
                    ));
                };
                let games = self.games.borrow();
                let Some(game) = games.get(&game_id) else {
                    error!("Could not find focused game {}", game_id);
                    return Some(Err(format!(
                        "Could not find focused game {game_id}! (Bot error!)"
                    )));
                };
                let bytes = target.as_bytes();
                let [c] = bytes[..] else {
                    return Some(Err(format!("Could not parse target {}", target)));
                };
                let Ok(target_id) = game.get_target(c).await else {
                    return Some(Err(format!("Could not find target {}", target)));
                };
                let target = target_id.map(|t| W(UserId(t)));
                Command::Game(game_id, GameCommand::Target { user_id, target })
            }
            ["reveal", ..] => {
                let user_id = W(user_id);
                let Some(game_id) = focus else {
                    return Some(Err(
                        "You have no focused game in which you can reveal".to_string()
                    ));
                };
                Command::Game(game_id, GameCommand::Reveal { user_id })
            }
            _ => return None,
        };
        Some(Ok(cmd))
    }
}
