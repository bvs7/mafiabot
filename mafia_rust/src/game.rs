mod error {
    use std::error::Error;
    use std::fmt::Display;

    #[derive(Debug, Clone)]
    pub struct ValidationErr {
        pub msg: String,
    }

    impl ValidationErr {
        pub fn new(msg: &str) -> Self {
            Self {
                msg: msg.to_string(),
            }
        }
    }

    impl Display for ValidationErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.msg)
        }
    }

    impl Error for ValidationErr {}
}

mod interface {
    use serde::{Deserialize, Serialize};
    use std::{
        fmt::Debug,
        sync::mpsc::{Receiver, Sender},
    };

    use super::game::{Actor, Ballot, Phase, Pidx, Player, RawPID, Target};
    // Eventually this will require a way to respond?
    pub trait Source: Debug + Clone + Default + Send {}

    /// Has details about where the command came from
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Request<U: RawPID, S: Source> {
        pub cmd: Command<U>,
        pub src: S,
        // Implementation specifics
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Command<U: RawPID> {
        Vote(U, Option<Ballot<U>>),
        Action(Actor<U>, Option<Target<U>>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Response<U: RawPID, S: Source> {
        pub event: Event<U>,
        pub src: S,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Event<U: RawPID> {
        Start {
            players: Vec<Player<U>>,
            phase: Phase,
        },
        Day,
        Vote {
            voter: Pidx,
            ballot: Option<Ballot<Pidx>>,
            former: Option<Ballot<Pidx>>,
            threshold: usize,
            count: usize,
        },
        Elect {
            ballot: Ballot<Pidx>,
        },
        Night,
        Action {
            actor: Actor<Pidx>,
            target: Option<Target<Pidx>>,
        },
        Dawn,
        Strip,
        Save,
        Investigate,
        Kill,
        Eliminate {
            player: Pidx,
        },
        Win,
        End,
        InvalidCommand,
    }

    #[derive(Debug)]
    pub struct Comm<U: RawPID, S: Source> {
        pub rx: Receiver<Request<U, S>>,
        pub tx: Sender<Response<U, S>>,
        pub src: S,
    }

    impl<U: RawPID, S: Source> Comm<U, S> {
        pub fn new(rx: Receiver<Request<U, S>>, tx: Sender<Response<U, S>>) -> Self {
            Self {
                rx,
                tx,
                src: S::default(),
            }
        }

        pub fn rx(&mut self) -> Command<U> {
            loop {
                let req = self.rx.recv();
                match req {
                    Err(err) => {
                        println!("Error: {:?}", err);
                        continue;
                    }
                    Ok(req) => {
                        self.src = req.src.clone();
                        return req.cmd;
                    }
                }
            }
        }
        pub fn tx(&self, event: Event<U>) {
            let resp = Response {
                event,
                src: self.src.clone(),
            };
            match self.tx.send(resp) {
                Err(err) => println!("Error: {:?}", err),
                Ok(_) => {}
            }
        }
    }
}

mod role {
    use super::game::RawPID;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
    pub enum Role<U: RawPID> {
        TOWN,
        COP,
        DOCTOR,
        CELEB,
        MILLER,
        MASON,
        MAFIA,
        GODFATHER,
        STRIPPER,
        GOON,
        IDIOT,
        SURVIVOR,
        GUARD(U),
        AGENT(U),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
    pub enum Team {
        Town,
        Mafia,
        Rogue,
    }
    impl<U: RawPID> Role<U> {
        pub fn team(&self) -> Team {
            match self {
                Role::TOWN | Role::COP | Role::DOCTOR | Role::CELEB => Team::Town,
                Role::MILLER | Role::MASON => Team::Town,
                Role::MAFIA | Role::GODFATHER | Role::GOON | Role::STRIPPER => Team::Mafia,
                Role::IDIOT | Role::SURVIVOR | Role::GUARD(_) | Role::AGENT(_) => Team::Rogue,
            }
        }
        pub fn investigate_mafia(&self) -> bool {
            match self {
                Role::GODFATHER => false,
                Role::MILLER => true,
                _ => self.team() == Team::Mafia,
            }
        }

        pub fn has_night_action(&self) -> bool {
            match self {
                Role::COP | Role::DOCTOR | Role::STRIPPER => true,
                _ => false,
            }
        }
    }
}

mod game {
    use serde::{Deserialize, Serialize};
    use std::fmt::{Debug, Display};
    use std::{
        sync::mpsc::{Receiver, Sender},
        thread::{self, JoinHandle},
    };

    use super::interface::{Comm, Event};
    use super::role::Role;
    use super::{
        error::ValidationErr,
        interface::{Command, Request, Response, Source},
        role::Team,
    };

    pub trait RawPID: Debug + Display + Clone + Copy + PartialEq + Eq + Send + Serialize {}

    pub type Pidx = usize;
    impl RawPID for Pidx {}

    #[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
    pub struct Player<U: RawPID> {
        pub raw_pid: U,
        pub name: String,
        pub role: Role<U>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
    pub enum Winner {
        Team(Team),
        Player(Pidx),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
    pub enum Ballot<U: RawPID> {
        Player(U),
        Abstain,
    }

    pub struct Election {
        electors: Vec<Pidx>,
        candidate: Pidx,
    }

    impl<U: RawPID> Display for Ballot<U> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Ballot::Player(p) => write!(f, "Player({})", p),
                Ballot::Abstain => write!(f, "Abstain"),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
    pub enum Actor<U: RawPID> {
        Player(U),
        Mafia(U),
    }
    impl<U: RawPID> Actor<U> {
        fn overlaps(&self, other: &Self) -> bool {
            match (self, other) {
                (Actor::Player(p1), Actor::Player(p2)) => p1 == p2,
                (Actor::Mafia(_), Actor::Mafia(_)) => true,
                _ => false,
            }
        }
        fn is_player(&self, p: U) -> bool {
            match self {
                Actor::Player(p2) => p == *p2,
                _ => false,
            }
        }
        fn is_mafia(&self) -> bool {
            match self {
                Actor::Mafia(_) => true,
                _ => false,
            }
        }
    }
    impl<U: RawPID> Display for Actor<U> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Actor::Player(p) => write!(f, "Player({})", p),
                Actor::Mafia(p) => write!(f, "Mafia({})", p),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
    pub enum Target<U: RawPID> {
        Player(U),
        NoTarget,
        Blocked,
    }

    pub type Votes = Vec<(Pidx, Ballot<Pidx>)>;
    pub type Actions = Vec<(Actor<Pidx>, Target<Pidx>)>;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
    pub enum Phase {
        Init,
        Day {
            day_no: usize,
            #[serde(skip)]
            votes: Votes,
        },
        Night {
            night_no: usize,
            #[serde(skip)]
            actions: Actions,
        },
        End(Winner),
    }

    impl Phase {
        pub fn clear(&mut self) {
            match self {
                Phase::Day { votes, .. } => votes.clear(),
                Phase::Night { actions, .. } => actions.clear(),
                _ => {}
            }
        }
        pub fn new_day(day_no: usize) -> Self {
            Self::Day {
                day_no,
                votes: Vec::new(),
            }
        }
        pub fn new_night(night_no: usize) -> Self {
            Self::Night {
                night_no,
                actions: Vec::new(),
            }
        }
    }

    // Want to ensure players can't be modified without clearing phase...
    type Players<U> = Vec<Player<U>>;

    #[derive(Debug, Serialize /*Deserialize*/)]
    pub struct Game<U: RawPID, S: Source> {
        players: Players<U>,
        phase: Phase,
        #[serde(skip)]
        comm: Comm<U, S>,
    }

    impl<U: RawPID, S: Source> Game<U, S> {
        pub fn new(
            players: Players<U>,
            rx: Receiver<Request<U, S>>,
            tx: Sender<Response<U, S>>,
        ) -> Self {
            let mut game = Self {
                players: Vec::new(),
                phase: Phase::Init,
                comm: Comm::new(rx, tx),
            };
            for player in players {
                // Todo print errors?
                Self::add_player(&mut game.players, &mut game.phase, player);
            }
            return game;
        }

        pub fn add_player(
            players: &mut Players<U>,
            phase: &Phase,
            player: Player<U>,
        ) -> Result<(), ValidationErr> {
            if let Phase::Init = phase {
                if Self::check_player(&players, &player.raw_pid).is_ok() {
                    return Err(ValidationErr::new("Player already exists"));
                }
                players.push(player);
                Ok(())
            } else {
                return Err(ValidationErr::new("Can't add player during game"));
            }
        }

        pub fn check_player(players: &Players<U>, raw_pid: &U) -> Result<Pidx, ValidationErr> {
            players
                .iter()
                .position(|p| p.raw_pid == *raw_pid)
                .map(|i: Pidx| i)
                .ok_or_else(|| ValidationErr {
                    msg: format!("Player {:?} not found", raw_pid),
                })
        }

        pub fn get_players_that(
            players: &Players<U>,
            f: fn((Pidx, Player<U>)) -> bool,
        ) -> impl Iterator<Item = (Pidx, Player<U>)> + '_ {
            players
                .iter()
                .enumerate()
                .map(|(i, p)| (i, p.clone()))
                .filter(move |(i, p)| f((*i, p.clone())))
        }
    }
    impl<U: RawPID + 'static, S: 'static + Source> Game<U, S> {
        pub fn start(mut self) -> Result<JoinHandle<()>, ()> {
            let even = self.players.len() % 2 == 0;
            match self.phase {
                Phase::Init if !even => self.phase = Phase::new_day(1),
                Phase::Init if even => self.phase = Phase::new_night(1),
                _ => return Err(()),
            };
            // Start game thread
            Ok(thread::spawn(move || self.game_thread()))
        }
    }

    impl<U: RawPID, S: Source> Game<U, S> {
        fn redo_game_thread(&mut self) {
            let players = &mut self.players;
            let comm = &mut self.comm;
            loop {
                match self.phase {
                    Phase::Init => {}
                    Phase::Day { day_no, votes } => match self.comm.rx() {
                        Command::Vote(v, b) => {
                            let (voter, ballot) = match Self::_validate_vote(players, v, b, comm) {
                                Ok((voter, ballot)) => (voter, ballot),
                                Err(e) => {
                                    self.comm.tx(Event::InvalidCommand);
                                    continue;
                                }
                            };
                            if let Some(election) =
                                Self::_accept_vote(&mut votes, voter, ballot, comm)
                            {
                                Self::_resolve_election(players, &mut self.phase, election, comm);
                            }
                        }
                        _ => {
                            self.comm.tx(Event::InvalidCommand);
                            continue;
                        }
                    },
                    Phase::Night { night_no, actions } => {}
                    Phase::End(winner) => {}
                }
            }
        }

        fn handle_day(players: &mut Players<U>, votes: &mut Votes, comm: &mut Comm<U, S>) {
            // Validate command
            let cmd = comm.rx();
            match cmd {
                Command::Vote(raw_voter, raw_ballot) => {
                    let election = match Self::_validate_vote(players, raw_voter, raw_ballot, comm)
                    {
                        Err(e) => {
                            comm.tx(Event::InvalidCommand);
                            None
                        }
                        Ok((voter, ballot)) => Self::_accept_vote(votes, voter, ballot, comm),
                    };
                    if let Some(election) = election {
                        Self::_resolve_election(players, &mut self.phase, election, comm);
                    }
                }
                _ => {}
            }

            // Handle command

            None
        }

        // SKELETON, IMPLEMENT!
        fn _validate_vote(
            players: &mut Players<U>,
            raw_voter: U,
            raw_ballot: Option<Ballot<U>>,
            comm: &mut Comm<U, S>,
        ) -> Result<(Pidx, Pidx), ValidationErr> {
            Err(ValidationErr {
                msg: "".to_string(),
            })
        }

        // SKELETON, IMPLEMENT!
        fn _accept_vote(
            votes: &mut Votes,
            voter: Pidx,
            ballot: Pidx,
            comm: &mut Comm<U, S>,
        ) -> Option<Election> {
            None
        }

        // SKELETON, IMPLEMENT!
        fn _resolve_election(
            players: &mut Players<U>,
            phase: &mut Phase,
            election: Election,
            comm: &mut Comm<U, S>,
        ) {
        }
    }
    impl<U: RawPID, S: Source> Game<U, S> {
        fn game_thread(&mut self) {
            Self::next_phase(&mut self.players, &mut self.phase, &self.comm);
            self.comm.tx(Event::Start {
                players: self.players.clone(),
                phase: self.phase.clone(),
            });

            loop {
                println!("Serialize: {}", serde_json::to_string(&self).unwrap());
                match &mut self.phase {
                    Phase::Day {
                        day_no,
                        ref mut votes,
                    } => {
                        let cmd = self.comm.rx();
                        match cmd {
                            Command::Vote(raw_voter, raw_ballot) => {
                                let elect = Self::handle_vote(
                                    &mut self.players,
                                    votes,
                                    raw_voter,
                                    raw_ballot,
                                    &self.comm,
                                );
                                match elect {
                                    None => {}
                                    Some(ballot) => {
                                        // "elect" subfn
                                        Self::handle_elect(
                                            &mut self.players,
                                            &mut self.phase,
                                            ballot,
                                            &self.comm,
                                        );
                                    }
                                }
                            }
                            _ => {
                                self.comm.tx(Event::InvalidCommand);
                            }
                        }
                    }

                    Phase::Night {
                        night_no,
                        ref mut actions,
                    } => {
                        let cmd = self.comm.rx();
                        match cmd {
                            Command::Action(raw_actor, raw_target) => {
                                if Self::handle_action(
                                    &mut self.players,
                                    actions,
                                    raw_actor,
                                    raw_target,
                                    &self.comm,
                                ) {
                                    let victim =
                                        Self::handle_dawn(&mut self.players, actions, &self.comm);

                                    match victim {
                                        None => {}
                                        Some(victim) => {
                                            Self::eliminate(
                                                &mut self.players,
                                                &mut self.phase,
                                                victim,
                                                &self.comm,
                                            );
                                        }
                                    };
                                    Self::next_phase(&self.players, &mut self.phase, &self.comm);
                                }
                            }
                            _ => {
                                self.comm.tx(Event::InvalidCommand);
                            }
                        }
                    }
                    Phase::End(winner) => {
                        break;
                    }
                    _ => {
                        self.comm.tx(Event::End);
                    }
                };
            }
            // Ok(())
        }

        fn handle_vote(
            players: &mut Players<U>,
            votes: &mut Votes,
            raw_voter: U,
            raw_ballot: Option<Ballot<U>>,
            comm: &Comm<U, S>,
        ) -> Option<Ballot<Pidx>> {
            match Self::validate_vote(players, raw_voter, raw_ballot, comm) {
                Err(err) => {
                    // Handle error response
                    comm.tx(Event::InvalidCommand);
                    None
                }
                Ok((voter, ballot)) => {
                    let former = Self::accept_vote(votes, voter, ballot, comm);
                    Self::check_elect(players, votes, former, comm)
                }
            }
        }

        fn validate_vote(
            players: &Players<U>,
            raw_voter: U,
            raw_ballot: Option<Ballot<U>>,
            comm: &Comm<U, S>,
        ) -> Result<(Pidx, Option<Ballot<Pidx>>), ValidationErr> {
            let voter = Self::check_player(players, &raw_voter)?;
            let ballot = match raw_ballot {
                Some(Ballot::Player(raw_pid)) => {
                    Some(Ballot::Player(Self::check_player(players, &raw_pid)?))
                }
                Some(Ballot::Abstain) => Some(Ballot::Abstain),
                None => None,
            };
            Ok((voter, ballot))
        }

        fn accept_vote(
            votes: &mut Votes,
            voter: Pidx,
            ballot: Option<Ballot<Pidx>>,
            comm: &Comm<U, S>,
        ) -> Option<Ballot<Pidx>> {
            let former = votes
                .iter()
                .position(|(v, _)| v == &voter)
                .map(|i| votes.remove(i));
            if let Some(ballot) = ballot {
                println!("Player {} votes for {:?}", voter, ballot);
                votes.push((voter, ballot));
            }
            former.map(|(v, b)| b)
        }

        fn check_elect(
            players: &Players<U>,
            votes: &Votes,
            former: Option<Ballot<Pidx>>,
            comm: &Comm<U, S>,
        ) -> Option<Ballot<Pidx>> {
            let n_players = players.len();
            let threshold = n_players / 2 + 1;
            let lo_thresh = (n_players + 1) / 2;

            if votes.len() == 0 {
                return None;
            }
            let (last_voter, last_ballot) = votes.last().unwrap();

            let threshold = match last_ballot {
                Ballot::Abstain => lo_thresh,
                _ => threshold,
            };

            let count = votes.iter().filter(|(_, b)| b == last_ballot).count();
            comm.tx(Event::Vote {
                voter: *last_voter,
                ballot: Some(*last_ballot),
                former,
                count,
                threshold,
            });
            match last_ballot {
                Ballot::Player(candidate) if count >= threshold => Some(Ballot::Player(*candidate)),
                Ballot::Abstain if count >= lo_thresh => Some(Ballot::Abstain),
                _ => None,
            }
        }

        fn handle_elect(
            players: &mut Players<U>,
            phase: &mut Phase,
            ballot: Ballot<Pidx>,
            comm: &Comm<U, S>,
        ) {
            comm.tx(Event::Elect { ballot });
            match ballot {
                Ballot::Player(elect) => {
                    Self::eliminate(players, phase, elect, comm);
                }
                Ballot::Abstain => {}
            };
            Self::next_phase(&players, phase, comm);
        }

        fn handle_action(
            players: &mut Players<U>,
            actions: &mut Actions,
            raw_actor: Actor<U>,
            raw_target: Option<Target<U>>,
            comm: &Comm<U, S>,
        ) -> bool {
            match Self::validate_action(players, raw_actor, raw_target, comm) {
                Err(err) => {
                    // Handle error response
                    comm.tx(Event::InvalidCommand);
                    false
                }
                Ok((actor, target)) => {
                    Self::accept_action(actions, actor, target, comm);
                    Self::check_dawn(players, actions, comm)
                }
            }
        }

        fn validate_action(
            players: &Players<U>,
            raw_actor: Actor<U>,
            raw_target: Option<Target<U>>,
            comm: &Comm<U, S>,
        ) -> Result<(Actor<Pidx>, Option<Target<Pidx>>), ValidationErr> {
            let actor = match raw_actor {
                Actor::Player(raw_pid) => Actor::Player(Self::check_player(players, &raw_pid)?),
                Actor::Mafia(raw_pid) => Actor::Mafia(Self::check_player(players, &raw_pid)?),
            };
            let target = match raw_target {
                Some(Target::Player(raw_pid)) => {
                    Some(Target::Player(Self::check_player(players, &raw_pid)?))
                }
                Some(Target::NoTarget) => Some(Target::NoTarget),

                None | Some(Target::Blocked) => None,
            };
            Ok((actor, target))
        }

        fn accept_action(
            actions: &mut Actions,
            actor: Actor<Pidx>,
            target: Option<Target<Pidx>>,
            comm: &Comm<U, S>,
        ) {
            // TODO: Role Check? Goon -> Target::Blocked?
            let former = actions
                .iter()
                .position(|(a, _)| a.overlaps(&actor))
                .map(|i| actions.remove(i));
            if let Some(target) = target {
                println!("Player {} acts on {:?}", actor, target);
                comm.tx(Event::Action {
                    actor,
                    target: Some(target),
                });
                actions.push((actor, target));
            }
        }

        fn check_dawn(players: &Players<U>, actions: &Actions, comm: &Comm<U, S>) -> bool {
            // Check that all possible actors have acted
            let actors = players
                .iter()
                .enumerate()
                .filter(|(_, p)| p.role.has_night_action())
                .map(|(i, _)| Actor::Player(i))
                .chain([Actor::Mafia(0)])
                .collect::<Vec<_>>();

            // For all actors, check that they have acted, or if Mafia, that at least one has acted
            for actor in actors {
                match actor {
                    Actor::Player(pid) => {
                        if actions.iter().find(|(a, _)| a == &actor).is_none() {
                            return false;
                        }
                    }
                    Actor::Mafia(_) => {
                        if actions.iter().find(|(a, _)| a.is_mafia()).is_none() {
                            return false;
                        }
                    }
                }
            }
            true
        }

        fn handle_dawn(
            players: &Players<U>,
            actions: &mut Actions,
            comm: &Comm<U, S>,
        ) -> Option<Pidx> {
            comm.tx(Event::Dawn);
            // Strip
            Self::get_players_that(players, |(_, p)| p.role == Role::STRIPPER)
                .for_each(|(stripped, _)| Self::strip(actions, stripped, comm));

            Self::get_players_that(players, |(_, p)| p.role == Role::DOCTOR)
                .for_each(|(saved, _)| Self::save(actions, saved, comm));

            let cops = Self::get_players_that(players, |(_, p)| p.role == Role::COP);
            for (cop, _) in cops {
                let suspect = actions
                    .iter()
                    .find(|(a, _)| a.is_player(cop))
                    .map(|(_, t)| t);
                if let Some(Target::Player(suspect)) = suspect {
                    Self::investigate(cop, *suspect, players, comm)
                }
            }

            let kill = actions.iter().find(|(a, _)| a.is_mafia());
            match kill {
                Some((a, Target::Player(victim))) => {
                    comm.tx(Event::Kill);
                    Some(*victim)
                }
                _ => None,
            }
        }

        fn eliminate(players: &mut Players<U>, phase: &mut Phase, victim: Pidx, comm: &Comm<U, S>) {
            comm.tx(Event::Eliminate { player: victim });
            println!("Eliminating player {}", victim);
            players.remove(victim);
            phase.clear();
            match Self::check_win(players, &comm) {
                None => {}
                Some(winner) => {
                    *phase = Phase::End(Winner::Team(winner));
                }
            }
        }

        fn next_phase(players: &Players<U>, phase: &mut Phase, comm: &Comm<U, S>) {
            match phase {
                Phase::Init => {
                    // TODO: set phase based on rules
                    *phase = Phase::Day {
                        day_no: 1,
                        votes: Vec::new(),
                    };
                }
                Phase::Day { day_no, .. } => {
                    println!("Day {} ends", day_no);
                    *phase = Phase::Night {
                        night_no: *day_no,
                        actions: Vec::new(),
                    };
                }
                Phase::Night { night_no, .. } => {
                    println!("Night {} ends", night_no);
                    *phase = Phase::Day {
                        day_no: *night_no + 1,
                        votes: Vec::new(),
                    };
                }
                _ => {}
            };
            match phase {
                Phase::Day { .. } => {
                    comm.tx(Event::Day);
                }
                Phase::Night { .. } => {
                    comm.tx(Event::Night);
                }
                Phase::End(_) => {
                    comm.tx(Event::End);
                }
                Phase::Init => {
                    panic!("Shouldn't ever next phase into Init")
                }
            };
        }

        fn check_win(players: &Players<U>, comm: &Comm<U, S>) -> Option<Team> {
            let n_players = players.len();
            let n_mafia = players
                .iter()
                .filter(|p| p.role.team() == Team::Mafia)
                .count();
            let result = match 0 {
                _ if n_mafia == 0 => Some(Team::Town),
                _ if n_players <= n_mafia * 2 => Some(Team::Mafia),
                _ => None,
            };
            if result.is_some() {
                comm.tx(Event::Win);
            }
            println!("Win condition: {:?}", result);
            result
        }

        fn strip(actions: &mut Actions, stripped: Pidx, comm: &Comm<U, S>) {
            for (actor, target) in actions {
                if actor == &Actor::Player(stripped) {
                    *target = Target::Blocked;

                    comm.tx(Event::Strip);
                }
            }
        }

        fn save(actions: &mut Actions, saved: Pidx, comm: &Comm<U, S>) {
            for (actor, target) in actions {
                if let Actor::Mafia(_) = actor {
                    *target = match target {
                        Target::Player(pid) if *pid == saved => Target::Blocked,
                        _ => *target,
                    };

                    comm.tx(Event::Save);
                }
            }
        }

        fn investigate(cop: Pidx, suspect: Pidx, players: &Players<U>, comm: &Comm<U, S>) {
            let is_mafia = players[suspect].role.investigate_mafia();

            comm.tx(Event::Investigate);
            // println!("Cop {:?} investigates {:?} and finds {:?}", cop, suspect, is_mafia);
        }
    }
}

mod test {
    use super::error::*;
    use super::game::*;
    use super::interface::*;
    use super::role::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn minimal() {
        impl RawPID for u64 {}
        impl Source for String {}
    }
}
