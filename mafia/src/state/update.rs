use crate::prelude::*;

use super::night_action::NightAct;
use super::util::{count_votes, thresh};
use super::{EventTx, StatusTx};

enum ElectionResult {}

impl State {
    pub fn update(&mut self, tx: &EventTx, update: &StatusTx) -> Option<DateTime<Local>> {
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
        update.send(self.clone());
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
        tx.send(Event::Election { choice, hammer, voters: voters.clone() });
        if let Some(pid) = choice {
            if self.players.get_role(pid) == Role::IDIOT {
                self.eclipse(pid, hammer, voters, tx);
                return;
            } else {
                self.eliminate(pid, hammer, self.context(), tx);
            }
        }
        if !self.check_end() {
            self.night(tx);
        }
    }

    pub fn dawn(&mut self, tx: &EventTx) {
        tx.send(Event::Dawn);
        let night_actions = NightAct::from_state(self);

        let blocks = self.apply_night_actions(night_actions, tx);
        if !self.check_end() {
            self.day(blocks, tx);
        }
    }

    pub fn kill(&mut self, mark: Pid, killer: Pid, tx: &EventTx) {
        self.eliminate(mark, killer, self.context(), tx);
    }

    pub fn eclipse(&mut self, avenger: Pid, hammer: Pid, guilty: Vec<Pid>, tx: &EventTx) {
        self.phase = Phase::Eclipse { avenger, hammer, guilty: guilty.clone() };
        tx.send(Event::Eclipse { avenger, hammer, guilty });
    }

    pub fn vengeance(&mut self, victim: Pid, avenger: Pid, hammer: Pid, tx: &EventTx) {
        self.eliminate(victim, avenger, self.context(), tx);
        self.eliminate(avenger, hammer, self.context(), tx);
        if !self.check_end() {
            self.night(tx);
        }
    }

    pub fn eliminate(&mut self, player: Pid, _culpable: Pid, context: Context, tx: &EventTx) {
        let role = self.players.eliminate(player, context);
        let role = role.kind();
        tx.send(Event::Eliminate { player, role, context });
    }

    pub fn day(&mut self, blocks: Blocks, tx: &EventTx) {
        self.day += 1;
        self.phase = Phase::Day { votes: HashMap::new(), blocks, elect: None };
        let counts = self.players.counts(Team::from);
        tx.send(Event::Day { day: self.day, counts });
    }

    pub fn night(&mut self, tx: &EventTx) {
        self.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };
        let counts = self.players.counts(Team::from);
        tx.send(Event::Night { day: self.day, counts });
    }

    pub fn check_end(&mut self) -> bool {
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
