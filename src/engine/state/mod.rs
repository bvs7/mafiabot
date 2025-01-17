#![allow(dead_code)]

pub mod phase;
pub mod players;
pub mod role;
pub mod rolegen;
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
use tokio::{
    net,
    sync::broadcast::{self, error::SendError},
};
use tracing::{debug, error, event, info, warn};

type Valid = std::result::Result<(), Error>;
type Result<T> = std::result::Result<T, SendError<Event>>;

pub type Choice = Option<u64>;
pub type Ballot = Option<Choice>;

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

            if matches!(action, Action::Reveal { .. }) && role != &Role::CELEB {
                return Err(Error::ExpectedCeleb {
                    actual: role.into(),
                });
            }

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
            players: self.players.alive().map(|(p, r)| (*p, *r)).collect(),
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

    pub fn vote(&mut self, voter: u64, ballot: Ballot, tx: &EventTx) -> Result<()> {
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

        Ok(())
    }

    pub fn reveal(&mut self, celeb: u64, tx: &EventTx) -> Result<()> {
        let Phase::Day { blocks, .. } = &self.phase else {
            panic!("Handling reveal when phase is not Day");
        };
        if let Some(blockers) = blocks.get(&celeb) {
            tx.send(Event::Block {
                blocked: celeb,
                blockers: blockers.clone(),
            })?;
        } else {
            tx.send(Event::Reveal { celeb })?;
        }
        Ok(())
    }
    pub fn scheme(&mut self, killer: u64, mark: Choice, tx: &EventTx) -> Result<()> {
        let Phase::Night { scheme, .. } = &mut self.phase else {
            panic!("Handling scheme when phase is not Night");
        };
        *scheme = Some((killer, mark));
        tx.send(Event::Scheme { killer, mark })?;
        Ok(())
    }

    pub fn target(&mut self, actor: u64, choice: Choice, tx: &EventTx) -> Result<()> {
        let Phase::Night { targets, .. } = &mut self.phase else {
            panic!("Handling scheme when phase is not Night");
        };
        targets.insert(actor, choice);
        tx.send(Event::Target { actor, choice })?;
        Ok(())
    }

    pub fn check_election(&self) -> Option<Choice> {
        let Phase::Day { votes, .. } = &self.phase else {
            warn!(msg = "Got check_election during not Day");
            return None;
        };
        let n = self.players.alive().count();

        let count = votes.iter().filter(|(_, c)| c.is_none()).count();
        if count >= (n + 1) / 2 {
            return Some(None);
        }

        let thresh = (n / 2) + 1;
        for (pid, _) in self.players.alive() {
            let pid = Some(*pid);
            let count = votes.iter().filter(|(_, p)| p == &&pid).count();
            if count >= thresh {
                return Some(pid);
            }
        }
        None
    }

    pub fn try_election(&mut self, choice: Choice, hammer: u64, tx: &EventTx) -> Result<()> {
        let Phase::Day { votes, .. } = &self.phase else {
            return Ok(()); // Can't elect when not in Day
        };
        let voters = votes
            .iter()
            .filter_map(|(p, c)| (&choice == c).then_some(*p))
            .collect();
        tx.send(Event::Election {
            choice,
            hammer,
            voters,
        })?;

        if let Some(elected) = choice {
            let context = Context {
                day: self.day,
                cause: Cause::Election,
            };
            self.eliminate(&elected, context, tx)?;
        }

        Ok(())
    }

    pub fn check_dawn(&self) -> bool {
        let Phase::Night { targets, scheme } = &self.phase else {
            return false;
        };
        if scheme.is_none() {
            return false;
        }
        let total = targets.len();
        let found = self
            .players
            .alive()
            .filter(|(_, r)| r.is_targeting())
            .count();
        if found < total {
            return false;
        }
        return true;
    }

    pub fn dawn(&mut self, tx: &EventTx) -> Result<()> {
        let Phase::Night { targets, scheme } = &self.phase else {
            panic!("Dawn when not night");
        };

        Ok(())
    }

    pub fn eliminate(&mut self, pid: &u64, context: Context, tx: &EventTx) -> Result<()> {
        let role = self.players.get(pid).unwrap();
        tx.send(Event::Eliminate {
            player: *pid,
            role: role.into(),
            context: context.clone(),
        })?;
        self.players.eliminate(pid, context);
        self.check_end(tx)?;
        Ok(())
    }

    pub fn check_end(&mut self, tx: &EventTx) -> Result<()> {
        let n = self.players.alive().count();
        let n_maf = self.players.alive().filter(|(_, r)| r.is_mafia()).count();
        let mut winner = None;
        if n_maf == 0 {
            winner = Some(Team::Town)
        } else if n - n_maf <= n_maf {
            winner = Some(Team::Mafia)
        }
        if let Some(winner) = winner {
            tx.send(Event::End { winner })?;
            self.phase = Phase::End { winner }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {}
