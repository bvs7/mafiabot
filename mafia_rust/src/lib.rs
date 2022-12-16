pub mod discord_;

mod rules {
    use super::game::{Day, Night, Step, UID};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub enum StartStep {
        Day,
        Night,
        #[default]
        NightIfEven,
        NightIfOdd,
    }

    impl StartStep {
        pub fn step<U: UID>(&self, len: usize) -> Step<U> {
            match self {
                StartStep::Night => Step::Night(Night::default()),
                StartStep::NightIfEven if len % 2 == 0 => Step::Night(Night::default()),
                StartStep::NightIfOdd if len % 2 == 1 => Step::Night(Night::default()),
                _ => Step::Day(Day::default()),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Rules {
        pub start_step: StartStep,
    }
}

mod game {
    use super::rules::*;

    use std::collections::HashMap;
    use std::error::Error;
    use std::fmt::{Debug, Display};
    use std::hash::Hash;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct MError(String);
    impl MError {
        pub fn new(msg: &str) -> Self {
            Self(msg.to_string())
        }
    }
    impl Display for MError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl Error for MError {}

    pub trait UID: Debug + Clone + PartialEq + Eq + Hash + Display {}

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum Role<U: UID> {
        TOWN,
        COP,
        DOCTOR,
        CELEB,
        MILLER,
        MASON,
        MAFIA,
        GODFATHER,
        GOON,
        IDIOT,
        SURVIVOR,
        GUARD(U),
        AGENT(U),
    }
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum Team {
        Town,
        Mafia,
        Rogue,
    }
    impl<U: UID> Role<U> {
        fn team(&self) -> Team {
            match self {
                Role::TOWN | Role::COP | Role::DOCTOR | Role::CELEB => Team::Town,
                Role::MILLER | Role::MASON => Team::Town,
                Role::MAFIA | Role::GODFATHER | Role::GOON => Team::Mafia,
                Role::IDIOT | Role::SURVIVOR | Role::GUARD(_) | Role::AGENT(_) => Team::Rogue,
            }
        }
    }
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Player<U: UID> {
        pub uid: U,
        pub role: Role<U>,
    }
    impl<U: UID> Player<U> {
        pub fn new(uid: &U, role: &Role<U>) -> Self {
            Self {
                uid: uid.to_owned(),
                role: role.to_owned(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Target<U: UID> {
        Select(U),
        Abstain,
    }
    #[derive(Debug, Clone)]
    pub enum Phase<U: UID> {
        Init,
        Play { day: u32, step: Step<U> },
        End,
    }

    impl<U: UID> Phase<U> {
        fn init(&self) -> Result<(), MError> {
            match self {
                Phase::Init => Ok(()),
                _ => Err(MError::new("Not in Init Phase")),
            }
        }
        fn play(&mut self) -> Result<&mut Step<U>, MError> {
            match self {
                Phase::Play { day, step } => Ok(step),
                _ => Err(MError::new("Not in Play Phase")),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Day<U: UID> {
        votes: HashMap<U, Target<U>>,
    }
    impl<U: UID> Default for Day<U> {
        fn default() -> Self {
            Self {
                votes: HashMap::new(),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Night<U: UID> {
        actions: HashMap<U, Target<U>>,
        mafia_target: Option<Target<U>>,
    }

    impl<U: UID> Default for Night<U> {
        fn default() -> Self {
            Self {
                actions: HashMap::new(),
                mafia_target: None,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub enum Step<U: UID> {
        Day(Day<U>),
        Night(Night<U>),
    }

    impl<U: UID> Step<U> {
        fn day(&mut self) -> Result<&mut Day<U>, MError> {
            match self {
                Step::Day(day) => Ok(day),
                _ => Err(MError::new("Not in Day Step")),
            }
        }
        fn night(&mut self) -> Result<&mut Night<U>, MError> {
            match self {
                Step::Night(night) => Ok(night),
                _ => Err(MError::new("Not in Night Step")),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Game<U: UID> {
        pub players: Vec<Player<U>>,
        pub phase: Phase<U>,
        pub rules: Rules,
    }

    impl<U: UID> Game<U> {
        pub fn new() -> Self {
            Self {
                players: Vec::new(),
                phase: Phase::Init,
                rules: Rules::default(),
            }
        }

        pub fn add_player(&mut self, player: Player<U>) -> Result<(), MError> {
            self.phase.init()?;
            if self.check_player(&player.uid).is_ok() {
                return Err(MError::new("Player already exists"));
            }
            self.players.push(player);
            Ok(())
        }

        pub fn check_player(&self, uid: &U) -> Result<&Player<U>, MError> {
            self.players
                .iter()
                .find(|p| p.uid == *uid)
                .ok_or(MError::new("Player does not exist"))
        }

        pub fn start(&mut self) -> Result<(), MError> {
            self.phase.init()?;
            // TODO: put this in start thread?
            self.phase = Phase::Play {
                day: 1,
                step: self.rules.start_step.step(self.players.len()),
            };
            Ok(())
        }

        pub fn vote(&mut self, voter: &U, target: &Option<Target<U>>) -> Result<(), MError> {
            self.phase.play()?.day()?;
            self.check_player(voter)?;
            let former = match target {
                None => self.phase.play()?.day()?.votes.remove(voter),
                Some(target) => match target {
                    Target::Select(uid) if self.check_player(uid).is_err() => {
                        return Err(MError::new("Target not playing"))
                    }
                    _ => self
                        .phase
                        .play()?
                        .day()?
                        .votes
                        .insert(voter.to_owned(), target.to_owned()),
                },
            };
            // TODO: Event!
            Ok(())
        }

        pub fn action(&mut self, actor: &U, target: &Option<Target<U>>) -> Result<(), MError> {
            self.phase.play()?.night()?;
            self.check_player(actor)?;
            let former = match target {
                None => self.phase.play()?.night()?.actions.remove(actor),
                Some(target) => match target {
                    Target::Select(uid) if self.check_player(uid).is_err() => {
                        return Err(MError::new("Target not playing"))
                    }
                    _ => self
                        .phase
                        .play()?
                        .night()?
                        .actions
                        .insert(actor.to_owned(), target.to_owned()),
                },
            };
            // TODO: Event!
            Ok(())
        }
    }
}

mod tests {
    use super::game::*;
    use super::rules::*;
    use std::fmt::Display;

    use crate::discord_::types::UserID;

    impl UID for UserID {}

    fn chkps<U: UID>(game: &Game<U>, players: &[U], should: bool) {
        assert_eq!(check_players(game, players, should), Ok(()));
    }

    fn check_players<U: UID>(game: &Game<U>, players: &[U], should: bool) -> Result<(), MError> {
        for player in players {
            match game.check_player(&player) {
                Ok(_) if should => Ok(()),
                Err(_) if !should => Ok(()),
                Ok(_) => Err(MError::new(&format!("Should not have player {}", player))),
                Err(_) => Err(MError::new(&format!("Should have player {}", player))),
            }?;
        }
        Ok(())
    }

    #[test]
    fn can_check_players() {
        let game = Game {
            players: vec![Player::new(&UserID(1), &Role::TOWN)],
            phase: Phase::Init,
            rules: Rules::default(),
        };

        assert!(game.check_player(&UserID(1)).is_ok());
    }

    #[test]
    fn can_add_players() {
        let mut game = Game::<UserID>::new();
        assert!(game.check_player(&UserID(1)).is_err());
        game.add_player(Player::new(&UserID(1), &Role::TOWN))
            .expect("Should add player");
        assert!(game.check_player(&UserID(1)).is_ok());
    }

    fn get_standard_game(uids: &[UserID]) -> Game<UserID> {
        let mut game = Game::<UserID>::new();
        let mut flag = true;
        for uid in uids {
            if flag {
                game.add_player(Player::new(uid, &Role::MAFIA))
                    .expect("Should add player");
                flag = false;
                continue;
            }
            game.add_player(Player::new(uid, &Role::TOWN))
                .expect("Should add player");
        }
        game
    }

    #[test]
    fn add_many_players() -> Result<(), MError> {
        let mut game = Game::<UserID>::new();

        let e1 = "Should not have player";
        let e2 = "Should have player";

        let (u1, u2, u3, u4) = (UserID(1), UserID(2), UserID(3), UserID(4));

        chkps(&game, &[u1], false);
        game.add_player(Player::new(&u1, &Role::TOWN))?;
        chkps(&game, &[u1], true);

        game.add_player(Player::new(&u2, &Role::COP))?;
        check_players(&game, &[u1, u2], true)?;
        check_players(&game, &[u3], false)?;

        game.add_player(Player::new(&u1, &Role::AGENT(u3.clone())))
            .expect_err("Should fail because u1 is already in the game");
        check_players(&game, &[u1, u2], true)?;
        check_players(&game, &[u3], false)?;

        game.add_player(Player::new(&u3, &Role::MAFIA))?;
        game.check_player(&u3)?;

        Ok(())
    }

    #[test]
    fn start_game() -> Result<(), MError> {
        let mut game = get_standard_game(&[UserID(1), UserID(2), UserID(3), UserID(4)]);
        game.start()?;
        Ok(())
    }

    #[test]
    fn vote() -> Result<(), MError> {
        let mut game = get_standard_game(&[UserID(1), UserID(2), UserID(3), UserID(4)]);
        game.start()?;
        // Fail night
        assert!(game
            .vote(&UserID(1), &Some(Target::Select(UserID(3))))
            .is_err());

        let mut game = get_standard_game(&[UserID(1), UserID(2), UserID(3)]);
        game.start()?;
        assert!(game
            .vote(&UserID(1), &Some(Target::Select(UserID(5))))
            .is_err());

        assert!(game
            .vote(&UserID(4), &Some(Target::Select(UserID(2))))
            .is_err());

        game.vote(&UserID(1), &Some(Target::Select(UserID(2))))?;

        Ok(())
    }
}
