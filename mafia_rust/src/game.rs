pub mod player;

pub mod comm;

use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use comm::*;
use player::*;
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

        game.comm.tx(Event::Init);

        for player in players {
            game.add_player(player);
        }
        return game;
    }

    pub fn add_player(&mut self, player: Player<U>) -> Result<(), String> {
        if let Phase::Init = self.phase {
            if !self.players.contains(&player) {
                Ok(self.players.push(player))
            } else {
                return Err("Player already exists".to_string());
            }
        } else {
            return Err("Can't add player during game".to_string());
        }
    }

    pub fn check_player(&self, raw_pid: &U) -> Result<Pidx, String> {
        self.players
            .iter()
            .position(|p| &p.raw_pid == raw_pid)
            .ok_or_else(|| "Player not found".to_string())
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
    fn game_thread(&mut self) {
        loop {
            match self.phase {
                Phase::Init => {}
                Phase::Day { .. } => self.handle_day(),
                Phase::Night { .. } => self.handle_night(),
                Phase::End(_) => {}
            }
            serde__json::to_file("game.json", self).unwrap();
        }
    }

    fn handle_day(&mut self) {
        let cmd = self.comm.rx();
        match cmd {
            Command::Vote(v, b) => self.handle_vote(v, b),
            _ => self.comm.tx(Event::InvalidCommand),
        }
    }

    fn handle_vote(&mut self, v: U, b: Ballot<U>) {
        // Validate vote
        let (voter, ballot) = match self.validate_vote(v, b) {
            Ok((voter, ballot)) => (voter, ballot),
            Err(e) => {
                self.comm.tx(Event::InvalidCommand);
                return;
            }
        };
        // accept vote
        let election = match self.accept_vote(voter, ballot) {
            Some(election) => election,
            None => return,
        };

        // resolve election
        self.resolve_election(election);
    }
    fn validate_vote(&mut self, v: U, b: Ballot<U>) -> Result<(Pidx, Ballot<Pidx>), String> {
        let voter = self.check_player(&v)?;
        let ballot = match b {
            Ballot::Player(raw_pid) => Ballot::Player(self.check_player(&raw_pid)?),
            Ballot::Abstain => Ballot::Abstain,
            Ballot::Retract => Ballot::Retract,
        };
        Ok((voter, ballot))
    }

    fn accept_vote(&mut self, voter: Pidx, ballot: Ballot<Pidx>) -> Option<Election> {
        let votes = match &mut self.phase {
            Phase::Day { votes, .. } => votes,
            _ => return None,
        };
        let former = votes
            .iter()
            .position(|(v, _)| v == &voter)
            .map(|i| votes.remove(i))
            .map(|(v, b)| b);

        match ballot {
            Ballot::Player(_) | Ballot::Abstain => votes.push((voter, ballot)),
            Ballot::Retract => {
                // EVENT RETRACT VOTE
                return None;
            }
        }

        let n_players = self.players.len();
        let threshold = match ballot {
            Ballot::Player(_) => n_players / 2 + 1,
            _ => (n_players + 1) / 2,
        };

        let electors = votes
            .iter()
            .filter(|(_, b)| b == &ballot)
            .map(|(v, _)| *v)
            .collect::<Vec<_>>();
        let count = electors.len();
        // EVENT VOTE
        if count >= threshold {
            Some(Election { electors, ballot })
        } else {
            None
        }
    }
    fn resolve_election(&mut self, election: Election) {
        let day_no = match self.phase {
            Phase::Day { day_no, .. } => day_no,
            _ => return,
        };
        // EVENT ELECTION
        match election.ballot {
            Ballot::Player(p) => {
                if let Some(winner) = self.eliminate(&p) {
                    return;
                }
            }
            _ => {
                // EVENT NO-ELIMINATION
            }
        }
        self.phase = Phase::new_night(day_no + 1);
    }

    fn handle_night(&mut self) {
        let cmd = self.comm.rx();
        match cmd {
            Command::Action(a, t) => self.handle_action(a, t),
            _ => self.comm.tx(Event::InvalidCommand),
        }
    }

    fn handle_action(&mut self, a: Actor<U>, t: Target<U>) {
        // Validate action
        let (actor, target) = match self.validate_action(a, t) {
            Ok((actor, target)) => (actor, target),
            Err(e) => {
                self.comm.tx(Event::InvalidCommand);
                return;
            }
        };

        // accept action
        match self.accept_action(actor, target) {
            Some(_) => {}
            None => return,
        };

        // resolve dawn
        self.resolve_dawn();
    }

    fn validate_action(
        &self,
        a: Actor<U>,
        t: Target<U>,
    ) -> Result<(Actor<Pidx>, Target<Pidx>), String> {
        let actor = match a {
            Actor::Player(raw_pid) => Actor::Player(self.check_player(&raw_pid)?),
            Actor::Mafia(raw_pid) => Actor::Mafia(self.check_player(&raw_pid)?),
        };
        let target = match t {
            Target::Player(raw_pid) => Target::Player(self.check_player(&raw_pid)?),
            Target::NoTarget => Target::NoTarget,
            Target::Blocked => Target::Blocked,
        };
        Ok((actor, target))
    }

    fn accept_action(&mut self, actor: Actor<Pidx>, target: Target<Pidx>) -> Option<()> {
        // Check for Goon
        let target = match actor {
            Actor::Mafia(p) if self.players[p].role == Role::GOON => Target::Blocked,
            _ => target,
        };

        let actions = match &mut self.phase {
            Phase::Night { actions, .. } => actions,
            _ => return None,
        };

        let former = actions
            .iter()
            .position(|(a, _)| a.overlaps(&actor))
            .map(|i| actions.remove(i));

        // EVENT ACTION
        actions.push((actor, target));

        let actor_count = self
            .players
            .iter()
            .filter(|p| p.role.has_night_action())
            .count();

        (actor_count == actions.len()).then(|| ())
    }

    fn resolve_dawn(&mut self) {
        // EVENT DAWN
        // Strip
        let (night_no, actions) = match &mut self.phase {
            Phase::Night { night_no, actions } => (*night_no, actions),
            _ => return,
        };

        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.role == Role::STRIPPER)
            .for_each(|(stripper, _)| Self::strip(actions, stripper, &self.comm));

        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.role == Role::DOCTOR)
            .for_each(|(doctor, _)| Self::save(actions, doctor, &self.comm));

        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.role == Role::COP)
            .for_each(|(cop, _)| Self::investigate(actions, cop, &self.comm));

        let kill = actions
            .iter()
            .find_map(|(a, t)| a.is_mafia().then_some((a, t)));

        if let Some((Actor::Mafia(killer), Target::Player(victim))) = kill {
            // EVENT KILL
            // (Copy to avoid borrow checker)
            let (killer, victim) = (killer.clone(), victim.clone());
            if let Some(winner) = self.eliminate(&victim) {
                return;
            }
        } else {
            // EVENT NO-KILL
        }

        self.phase = Phase::new_day(night_no + 1);
    }

    fn strip(actions: &mut Actions, stripper: Pidx, comm: &Comm<U, S>) {
        for (actor, target) in actions {
            if actor.is_player(stripper) {
                // EVENT STRIP
                *target = Target::Blocked;
            }
        }
    }

    fn save(actions: &mut Actions, doctor: Pidx, comm: &Comm<U, S>) {
        for (actor, target) in actions {
            if actor.is_mafia() && target.is_player(doctor) {
                // EVENT SAVE
                *target = Target::Blocked;
            }
        }
    }

    fn investigate(actions: &mut Actions, cop: Pidx, comm: &Comm<U, S>) {
        for (actor, target) in actions {
            if actor.is_player(cop) {
                match target {
                    Target::Player(suspect) => {
                        // EVENT INVESTIGATE
                    }
                    _ => {}
                }
            }
        }
    }

    fn eliminate(&mut self, p: &Pidx) -> Option<Winner> {
        // EVENT ELIMINATE
        self.players.remove(p.clone());
        // all Pidxs are now invalid...
        self.phase.clear();

        return self.check_win();
    }

    fn check_win(&mut self) -> Option<Winner> {
        let n_players = self.players.len();
        let n_mafia = self
            .players
            .iter()
            .filter(|p| p.role.team() == Team::Mafia)
            .count();

        let winner = match 0 {
            _ if n_mafia == 0 => Some(Winner::Team(Team::Town)),
            _ if n_mafia >= n_players => Some(Winner::Team(Team::Mafia)),
            _ => None,
        };
        if let Some(winner) = winner {
            self.phase = Phase::End(winner);
            // END PHASE
        }
        winner
    }
}
