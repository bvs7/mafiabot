#![allow(dead_code)]

pub mod phase;
pub mod players;
pub mod role;
pub mod rules;

use super::interface::{
    Action, ActionMsg, ActionRx, ActionTx, CountKey, CountKeyKind, Election, Error, Event, EventRx,
    EventTx,
};
use phase::{Phase, PhaseKind};
use players::{Cause, Context, PlayerLog, PlayerState, Players};
use role::{Role, RoleKind, Team};
use rules::Rules;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{self, error::SendError};
use tracing::{debug, error, event, info};

type Valid = std::result::Result<(), Error>;
type Result<T> = std::result::Result<T, SendError<Event>>;

type Choice = Option<u64>;
type Ballot = Option<Choice>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    id: u64,
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
}

impl State {
    pub fn new<'a>(
        id: u64,
        registry: impl IntoIterator<Item = &'a (u64, Role)>,
        rules: Rules,
    ) -> Self {
        Self {
            id,
            day: 0,
            phase: Phase::Init,
            players: registry.into(),
            rules,
        }
    }

    fn counts(&self, key_kind: CountKeyKind) -> HashMap<CountKey, usize> {
        let mut counts = HashMap::new();
        let key = |role: &Role| match key_kind {
            CountKeyKind::Role => role.kind().into(),
            CountKeyKind::Team => role.team().into(),
            CountKeyKind::IsMafia => role.is_mafia().into(),
            CountKeyKind::Players => CountKey::Players,
        };

        for (_, role) in self.players.alive() {
            *counts.entry(key(role)).or_insert(0) += 1;
        }
        counts
    }

    pub fn validate_action(&self, action: &Action) -> Valid {
        info!("Validating action {:?}", action);
        let expected = match action {
            Action::Start => Some(PhaseKind::Init),
            Action::Vote { .. } | Action::Reveal { .. } => Some(PhaseKind::Day),
            Action::Target { .. } | Action::Scheme { .. } => Some(PhaseKind::Night),
        };
        if let Some(expected) = expected {
            let actual: PhaseKind = (&self.phase).into();
            if expected != actual {
                return Err(Error::InvalidPhase { actual, expected });
            }
        }

        if let Some(actor) = action.actor() {
            let role = self
                .players
                .get(&actor)
                .ok_or(Error::InvalidActor { pid: actor })?;

            if matches!(action, Action::Target { .. }) && !role.is_targeting() {
                return Err(Error::ExpectedTargetingRole {
                    actual: role.into(),
                });
            }

            if matches!(action, Action::Scheme { .. }) && !role.is_scheming() {
                return Err(Error::ExpectedSchemingRole {
                    actual: role.into(),
                });
            }

            if let Some(other) = action.other() {
                let _ = *self
                    .players
                    .get(&other)
                    .ok_or(Error::InvalidOther { pid: other })?;
            }

            if let Action::Vote { voter, ballot } = action {
                if let Phase::Day { votes, .. } = &self.phase {
                    let current = votes.get(&voter);
                    if current == ballot.as_ref() {
                        return Err(Error::IneffectiveVote);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn start(&mut self, tx: &EventTx) -> Result<()> {
        // If number of players is odd, start day, if even, start night
        tx.send(Event::Start {
            id: self.id,
            players: self.players.alive().map(|(p, _)| *p).collect(),
            rules: self.rules.clone(),
            counts: self.counts(CountKeyKind::Team), // TODO: set with rules
        })?;

        if self.players.alive().count() % 2 == 1 {
            self.phase = Phase::new(PhaseKind::Day);
            self.day += 1;
            tx.send(Event::Day {
                day: self.day,
                counts: self.counts(CountKeyKind::Team), // TODO: set with rules
            })?;
        } else {
            self.phase = Phase::new(PhaseKind::Night);
            tx.send(Event::Night {
                day: self.day,
                counts: self.counts(CountKeyKind::Team),
            })?; // TODO: set with rules
        }
        Ok(())
    }

    pub fn vote(&mut self, voter: u64, ballot: Ballot, tx: &EventTx) -> Result<Option<Election>> {
        let Phase::Day { votes, .. } = &mut self.phase else {
            panic!("Handling vote when phase is not Day");
        };

        let former = if let Some(choice) = ballot {
            votes.insert(voter, choice)
        } else {
            votes.remove(&voter)
        };

        struct Check {
            choice: Choice,
            voters: Vec<u64>,
        }

        fn count(c: &Check) -> (Choice, usize) {
            (c.choice, c.voters.len())
        }

        let check = |choice: Choice| Check {
            choice,
            voters: votes
                .iter()
                .filter_map(|(p, c)| (c == &choice).then_some(*p))
                .collect(),
        };

        let ballot_check = ballot.map(check);
        let former_check = former.map(check);

        tx.send(Event::Vote {
            voter,
            ballot: ballot_check.as_ref().map(count),
            former: former_check.as_ref().map(count),
        })?;

        let hammer = voter;
        let n = self.players.alive().count();
        let thresh = (n / 2) + 1;
        let pthresh = (n + 1) / 2;

        let check_elect = |Check { choice, voters }: Check| {
            let t = if choice.is_some() { thresh } else { pthresh };
            (voters.len() >= t).then_some(Election {
                choice,
                hammer,
                voters,
            })
        };

        Ok(ballot_check
            .map(check_elect)
            .flatten()
            .or_else(|| former_check.map(check_elect).flatten()))
    }

    pub fn get_election(&self, choice: Choice, thresh: usize, hammer: u64) -> Option<Election> {
        let Phase::Day { votes, .. } = &self.phase else {
            return None;
        };
        let voters: Vec<_> = votes
            .iter()
            .filter_map(|(p, c)| (c == &choice).then(|| *p))
            .collect();
        if voters.len() < thresh {
            return None;
        }
        Some(Election {
            choice,
            hammer,
            voters,
        })
    }

    pub fn election(&mut self, tx: &EventTx) -> Result<()> {
        // find election? Or should it already be here?

        // tx.send(Event::Election(elect.clone()))?;
        // let Election {
        //     choice,
        //     hammer,
        //     voters,
        // } = elect.clone();
        // if let Some(pid) = choice {
        //     self.eliminate(&pid, (self.day, elect).into(), tx);
        // }

        // // to night
        // self.phase = Phase::Night {
        //     targets: HashMap::new(),
        //     scheme: None,
        // };
        // let _ = tx.send(Event::Night {
        //     day: self.day,
        //     counts: self.counts(CountKeyKind::Team),
        // })?;
        Ok(())
    }

    pub fn eliminate(&mut self, pid: &u64, context: Context, tx: &EventTx) -> Result<Option<Team>> {
        tx.send(Event::Eliminate {
            player: *pid,
            context: context.clone(),
        })?;
        self.players.eliminate(pid, context);
        // Check to see if the game has ended
        let n = self.players.alive().count();
        let n_maf = self.players.alive().filter(|(_, r)| r.is_mafia()).count();

        Ok(if n_maf == 0 {
            Some(Team::Town)
        } else if (n - n_maf) <= n_maf {
            Some(Team::Mafia)
        } else {
            None
        })
    }

    // pub fn check_dawn(&self) -> Result<bool> {
    //     let Phase::Night { targets, scheme } = &self.phase else {};
    //     let total = self
    //         .players
    //         .alive()
    //         .filter(|(_, r)| r.is_targeting())
    //         .count();
    //     Ok(scheme.is_some() && targets.len() == total)
    // }
}

#[cfg(test)]
mod test {}
