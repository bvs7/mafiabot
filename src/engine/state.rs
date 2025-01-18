use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio::{
    sync::{broadcast, oneshot, RwLock},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument, trace, warn};

pub mod phase;
pub mod players;
pub mod role;
pub mod rules;

use super::interface::*;
use phase::*;
use players::*;
use role::*;
use rules::*;

pub type Choice = Option<u64>;
pub type Ballot = Option<Choice>;

pub struct State {
    id: u64,
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    event_tx: EventTx,
    undo_timer: Option<JoinHandle<()>>,
}

impl State {
    pub fn new(id: u64, registry: Vec<(u64, Role)>, rules: Rules) -> Self {
        let event_tx = broadcast::Sender::new(100);
        Self {
            id,
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(&registry),
            rules,
            event_tx,
            undo_timer: None,
        }
    }

    #[instrument(skip_all)]
    pub async fn action_handler(this: Arc<RwLock<Self>>, action_rx: &mut ActionRx) {
        while let Some((action, resp)) = action_rx.recv().await {
            Self::handle_action(&this, action, resp);
        }
    }

    /// Broadcast an Event if anyone is listening
    pub fn tx(&self, event: Event) {
        if self.event_tx.receiver_count() > 0 {
            let _ = self.event_tx.send(event);
        }
    }

    pub fn subscribe(&self) -> EventRx {
        self.event_tx.subscribe()
    }

    /// Check if an action would be valid to perform
    pub fn validate_action(&self, action: Action) -> Result<(), Error> {
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

    // The game pointer is used for... Event tx, to send events...
    // And for timers to have a handle on the state mutex...
    // It would be nice to do this differently...
    pub async fn handle_action(
        this: &Arc<RwLock<Self>>,
        action: Action,
        resp: oneshot::Sender<Result<(), Error>>,
    ) -> anyhow::Result<()> {
        // TODO: Check if locking read, validating, then on Ok locking write and re-validating would work?
        let mut wstate = this.write().await;
        // TODO: should this return anything else?
        let result = wstate.validate_action(action);
        resp.send(result.clone());

        enum Result {
            Election { choice: Choice, hammer: u64 },
            Dawn,
        }

        let result = match action {
            Action::Start => {
                wstate.start();
                None
            }
            Action::Vote { voter, ballot } => {
                let result = wstate.vote(voter, ballot);
                let reached_new_election = Some(Some(1));
                let hammer = voter;

                if let Some(choice) = reached_new_election {
                    Some(Result::Election { choice, hammer })
                } else {
                    None
                }
            }
            Action::Target { actor, choice } => {
                // ...
                let night_done = true;
                if night_done {
                    Some(Result::Dawn)
                } else {
                    None
                }
            }
            Action::Scheme { killer, mark } => {
                // ...
                let night_done = true;
                if night_done {
                    Some(Result::Dawn)
                } else {
                    None
                }
            }
            _ => None,
        };

        // IF DAWN OR ELECTION IS DETECTED
        match result {
            Some(Result::Election { choice, hammer }) => {
                let h = tokio::spawn(Self::election_timer(this.clone(), choice, hammer));
                if let Some(old_h) = wstate.undo_timer.replace(h) {
                    old_h.abort();
                }
            }
            Some(Result::Dawn) => {
                let h = tokio::spawn(Self::dawn_timer(this.clone()));
                if let Some(old_h) = wstate.undo_timer.replace(h) {
                    old_h.abort();
                }
            }
            None => {}
        }

        drop(wstate);

        todo!()
    }

    fn start(&mut self) {
        let _ = self.event_tx.send(Event::Start {
            id: self.id,
            players: self.players.alive(),
            rules: self.rules.clone(),
            counts: self.counts(Team::from),
        });
    }

    fn vote(&mut self, voter: u64, ballot: Ballot) -> Result<Option<Vec<u64>>, Error> {
        todo!()
    }

    fn reveal(&self, actor: u64) -> Result<(), Error> {
        todo!()
    }

    fn target(&mut self, actor: u64, choice: Choice) -> Result<bool, Error> {
        Ok(self.check_dawn())
    }

    fn scheme(&mut self, killer: u64, mark: Choice) -> Result<bool, Error> {
        Ok(self.check_dawn())
    }

    fn check_election(&self, choice: Choice) -> Option<(Vec<u64>)> {
        todo!()
    }

    async fn election_timer(this: Arc<RwLock<Self>>, choice: Choice, hammer: u64) {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let mut wstate = this.write().await;
        // TODO: Error tracing
        if let Some(users) = wstate.check_election(choice) {
            wstate.election(choice, hammer, users);
        }
        wstate.undo_timer = None;
    }

    fn election(&mut self, choice: Choice, hammer: u64, voters: Vec<u64>) {
        todo!()
    }

    fn check_dawn(&self) -> bool {
        todo!()
    }

    async fn dawn_timer(this: Arc<RwLock<Self>>) {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let mut wstate = this.write().await;
        // TODO: Error tracing
        wstate.dawn();
        wstate.undo_timer = None;
    }

    fn dawn(&mut self) {
        todo!()
    }

    fn counts<C, F>(&self, f: F) -> HashMap<C, usize>
    where
        C: std::hash::Hash + Eq,
        F: Fn(Role) -> C,
    {
        let mut counts = HashMap::new();
        for (_, role) in self.players.alive() {
            *counts.entry(f(role)).or_insert(0) += 1;
        }
        counts
    }
}
