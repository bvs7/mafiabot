pub mod player;

pub mod comm;

mod test;

use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::prelude::*;
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
    pub fn new(players: Players<U>, comm: Comm<U, S>) -> Self {
        let mut game = Self {
            players: Vec::new(),
            phase: Phase::Init,
            comm,
        };

        game.comm.tx(Event::Init);

        for player in players {
            if let Err(_) = game.add_player(player) {
                continue;
            }
        }
        return game;
    }

    pub fn add_player(&mut self, player: Player<U>) -> Result<(), ()> {
        if let Phase::Init = self.phase {
            if !self
                .players
                .iter()
                .map(|p| &p.raw_pid)
                .any(|pid| pid == &player.raw_pid)
            {
                Ok(self.players.push(player))
            } else {
                self.comm
                    .tx(Event::InvalidCommand("Player already exists".to_string()));
                return Err(());
            }
        } else {
            self.comm.tx(Event::InvalidCommand(
                "Can't add player during game".to_string(),
            ));
            return Err(());
        }
    }

    pub fn check_player(&self, raw_pid: &U) -> Result<Pidx, String> {
        self.players
            .iter()
            .position(|p| &p.raw_pid == raw_pid)
            .ok_or_else(|| "Player not found".to_string())
    }

    pub fn save_game(&self, fname: &str) -> Result<(), ()> {
        let mut f = File::create(fname).map_err(|_| ())?;
        serde_json::to_writer_pretty(&mut f, &self).map_err(|_| ())?;
        Ok(())
    }
}

impl<U: RawPID + 'static, S: 'static + Source> Game<U, S> {
    pub fn start(mut self) -> Result<JoinHandle<Self>, Self> {
        if self.players.len() < 3 {
            self.comm.tx(Event::InvalidCommand(
                "Can't start game with less than 3 players".to_string(),
            ));
            return Err(self);
        }
        if let Some(_) = self.check_team_numbers() {
            self.comm.tx(Event::InvalidCommand(
                "Can't start game with given roles".to_string(),
            ));
            return Err(self);
        }

        let even = self.players.len() % 2 == 0;
        match self.phase {
            Phase::Init if !even => self.phase = Phase::new_day(1),
            Phase::Init if even => self.phase = Phase::new_night(1),
            _ => return Err(self),
        };
        self.comm.tx(Event::Start {
            players: self.players.clone(),
            phase: self.phase.clone(),
        });
        // Start game thread
        Ok(thread::spawn(move || self.game_thread()))
    }
}

impl<U: RawPID, S: Source> Game<U, S> {
    fn game_thread(mut self) -> Self {
        loop {
            match self.phase {
                Phase::Init => {}
                Phase::Day { .. } => self.handle_day(),
                Phase::Night { .. } => self.handle_night(),
                Phase::End(_) => {
                    self.comm.tx(Event::End);
                    break;
                }
            }
            if let SaveStrategy::PerChange(fname) = &self.comm.save {
                self.save_game(fname).expect("Saving game should work");
            };
        }
        self
    }

    fn handle_day(&mut self) {
        let cmd = self.comm.rx();
        match cmd {
            Command::Vote(v, b) => self.handle_vote(v, b),
            Command::End => self.phase = Phase::End(Winner::None),
            _ => {
                self.comm.tx(Event::InvalidCommand(
                    "Invalid command for Day Phase".to_string(),
                ));
            }
        }
    }

    fn handle_vote(&mut self, v: U, b: Ballot<U>) {
        // Validate vote
        let (voter, ballot) = match self.validate_vote(v, b) {
            Ok((voter, ballot)) => (voter, ballot),
            Err(e) => {
                self.comm.tx(Event::InvalidCommand(e));
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
                self.comm.tx(Event::RetractVote { voter, former });
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

        self.comm.tx(Event::Vote {
            voter,
            ballot,
            former,
            count,
            threshold,
        });

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
        self.comm.tx(Event::Election {
            electors: election.electors.clone(),
            ballot: election.ballot.clone(),
        });
        match election.ballot {
            Ballot::Player(p) => {
                if let Some(winner) = self.eliminate(&p) {
                    return;
                }
            }
            _ => {}
        }
        self.next_phase(Phase::new_night(day_no + 1));
        self.phase = Phase::new_night(day_no + 1);
    }

    fn handle_night(&mut self) {
        let cmd = self.comm.rx();
        match cmd {
            Command::Action(a, t) => self.handle_action(a, t),
            Command::End => self.phase = Phase::End(Winner::None),
            _ => {}
        }
    }

    fn handle_action(&mut self, a: Actor<U>, t: Target<U>) {
        // Validate action
        let (actor, target) = match self.validate_action(a, t) {
            Ok((actor, target)) => (actor, target),
            Err(e) => {
                self.comm.tx(Event::InvalidCommand(e));
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
        match actor {
            Actor::Player(p) if !self.players[p].role.has_night_action() => {
                return Err("Player does not have a night action".to_string());
            }
            Actor::Mafia(p) if self.players[p].role.team() != Team::Mafia => {
                return Err("Only Mafia Players can kill at ngiht".to_string());
            }
            _ => {}
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

        self.comm.tx(Event::Action { actor, target });
        actions.push((actor, target));

        let actor_count = self
            .players
            .iter()
            .filter(|p| p.role.has_night_action())
            .count()
            + 1; // Mafia action

        (actor_count == actions.len()).then(|| ())
    }

    fn resolve_dawn(&mut self) {
        self.comm.tx(Event::Dawn);
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
            .for_each(|(cop, _)| Self::investigate(&self.players, actions, cop, &self.comm));

        let kill = actions
            .iter()
            .find_map(|(a, t)| a.is_mafia().then_some((a, t)));

        if let Some((Actor::Mafia(killer), Target::Player(victim))) = kill {
            // (Copy to avoid borrow checker)
            let (killer, victim) = (killer.clone(), victim.clone());
            self.comm.tx(Event::Kill { killer, victim });
            if let Some(winner) = self.eliminate(&victim) {
                return;
            }
        } else {
            self.comm.tx(Event::NoKill);
        }

        self.phase = Phase::new_day(night_no + 1);
    }

    fn strip(actions: &mut Actions, stripper: Pidx, comm: &Comm<U, S>) {
        // Get stripped Pidx
        let stripped = actions
            .iter()
            .find_map(|(a, t)| a.is_player(stripper).then_some(t));

        let stripped = match stripped {
            Some(Target::Player(p)) => *p,
            _ => return,
        };

        // Find strippeds action
        for (action, target) in actions {
            if action.is_player(stripped) {
                comm.tx(Event::Strip { stripper, stripped });
                *target = Target::Blocked;
                return;
            }
        }
    }

    fn save(actions: &mut Actions, doctor: Pidx, comm: &Comm<U, S>) {
        // Get saved
        let saved = actions
            .iter()
            .find_map(|(a, t)| a.is_player(doctor).then_some(t));

        let saved = match saved {
            Some(Target::Player(p)) => *p,
            _ => return,
        };

        // Find Mafia Action
        for (action, target) in actions {
            if action.is_mafia() && target.is_player(saved) {
                comm.tx(Event::Save { doctor, saved });
                *target = Target::Blocked;
                return;
            }
        }
    }

    fn investigate(players: &Players<U>, actions: &mut Actions, cop: Pidx, comm: &Comm<U, S>) {
        for (actor, target) in actions {
            if actor.is_player(cop) {
                match target {
                    Target::Player(suspect) => {
                        let suspect = suspect.clone();
                        let role = players[suspect].role;
                        comm.tx(Event::Investigate { cop, suspect, role });
                    }
                    _ => {}
                }
            }
        }
    }

    fn eliminate(&mut self, p: &Pidx) -> Option<Winner> {
        self.comm.tx(Event::Eliminate { player: *p });
        self.players.remove(p.clone());
        // all Pidxs are now invalid...
        self.phase.clear();

        return self.check_team_win();
    }

    fn next_phase(&mut self, next_phase: Phase) {
        self.phase = next_phase;

        if let SaveStrategy::PerPhase(fname) = &self.comm.save {
            self.save_game(fname).expect("Saving game should work");
        };
    }

    fn check_team_numbers(&self) -> Option<Winner> {
        let n_players = self.players.len();
        let n_mafia = self
            .players
            .iter()
            .filter(|p| p.role.team() == Team::Mafia)
            .count();

        match 0 {
            _ if n_mafia == 0 => Some(Winner::Team(Team::Town)),
            // True if: 3/5, 3/6. False if: 2/5, 2/6
            _ if n_mafia > (n_players - 1) / 2 => Some(Winner::Team(Team::Mafia)),
            _ => None,
        }
    }

    fn check_team_win(&mut self) -> Option<Winner> {
        let winner = self.check_team_numbers();

        if let Some(winner) = winner {
            self.comm.tx(Event::Win { winner });
            self.phase = Phase::End(winner);
        }
        winner
    }
}
