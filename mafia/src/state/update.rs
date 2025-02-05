use crate::prelude::*;

use super::night_action::NightAct;
use super::util::{count_votes, thresh};
use super::EventTx;

enum ElectionResult {}

impl State {
    pub fn update(&mut self, tx: &EventTx) -> Option<DateTime<Local>> {
        let result = match &self.phase {
            Phase::Day { elect: Some((_, _, time)), .. } if time < &Local::now() => {
                self.election(tx);
                None
            }
            Phase::Night { dawn: Some(time), .. } if time < &Local::now() => {
                self.dawn(tx);
                None
            }
            Phase::Day { elect: Some((_, _, time)), .. }
            | Phase::Night { dawn: Some(time), .. } => Some(*time),
            _ => None,
        };
        return None;
    }

    pub fn context(&self) -> Context {
        Context::new(self.day, self.phase.kind())
    }

    pub fn election(&mut self, tx: &EventTx) {
        let Phase::Day { votes, elect: Some((choice, hammer, _)), .. } = &self.phase else {
            panic!("Expected Day phase with some election");
        };
        let (choice, hammer) = (*choice, *hammer);
        let vote_list = count_votes(votes, &self.players);
        let th = thresh(self.players.alive().len(), &choice);
        let ph = thresh(self.players.alive().len(), &None);
        let voters = count_votes(votes, &self.players).get(&choice).unwrap().clone();
        let _ = tx.send(Event2::Election { choice, hammer, vote_list });
        if let Some(pid) = choice {
            if self.players.get_role(pid) == Role::IDIOT {
                self.eclipse(pid, hammer, voters, tx);
                return;
            } else {
                let context = self.context();
                self.eliminate(pid, hammer, context, tx);
            }
        }
        if !self.check_end(tx) {
            self.night(tx);
        }
    }

    pub fn dawn(&mut self, tx: &EventTx) {
        let night_actions = NightAct::from_state(&self);
        let _ = tx.send(Event2::Dawn { night_actions: night_actions.clone() });

        let blocks = self.apply_night_actions(night_actions, tx);
        if !self.check_end(tx) {
            self.day(blocks, tx);
        }
    }

    pub fn kill(&mut self, mark: Pid, killer: Pid, tx: &EventTx) {
        let context = self.context();
        self.eliminate(mark, killer, context, tx);
    }

    pub fn eclipse(&mut self, avenger: Pid, hammer: Pid, guilty: Vec<Pid>, tx: &EventTx) {
        self.phase = Phase::Eclipse { avenger, hammer, guilty: guilty.clone(), vengeance: None };
        let _ = tx.send(Event2::Eclipse { avenger, hammer, guilty });
    }

    // TODO: just set a variable for this, don't execute until update
    pub fn vengeance(&mut self, victim: Pid, avenger: Pid, hammer: Pid, tx: &EventTx) {
        let context = self.context();
        self.eliminate(victim, avenger, context.clone(), tx);
        self.eliminate(avenger, hammer, context, tx);
        if !self.check_end(tx) {
            self.night(tx);
        }
    }

    pub fn eliminate(&mut self, player: Pid, _culpable: Pid, context: Context, tx: &EventTx) {
        let role = self.players.eliminate(player, context);
        let role = role.kind();
        let _ = tx.send(Event2::Eliminate { player, role });
    }

    pub fn day(&mut self, blocks: Blocks, tx: &EventTx) {
        self.day += 1;
        self.phase = Phase::Day { votes: HashMap::new(), blocks, elect: None };
        let players = self.players.alive();
        let _ = tx.send(Event2::Day { day: self.day, players });
    }

    pub fn night(&mut self, tx: &EventTx) {
        self.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };
        let players = self.players.alive();
        let _ = tx.send(Event2::Night { day: self.day, players });
    }

    pub fn check_end(&mut self, tx: &EventTx) -> bool {
        let counts = self.players.counts(Team::from);
        let n = self.players.alive().len();
        let n_maf = counts.get(&Team::Mafia).copied().unwrap_or(0);
        if n_maf == 0 {
            self.phase = Phase::End { winner: Team::Town };
            return true;
        } else if n - n_maf <= n_maf {
            self.phase = Phase::End { winner: Team::Mafia };
            return true;
        }
        false
    }
}
