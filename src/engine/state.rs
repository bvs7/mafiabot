#![allow(dead_code)]

mod builder;

use std::{
    collections::HashMap,
    hash::Hash,
    io::{Read, Write},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{broadcast, Mutex, Notify, RwLock},
    task::JoinHandle,
};
use tracing::{debug, error, event, info};

use crate::engine::interface::Election;

use super::{
    interface::{ActionRx, ActionTx},
    Action, Error, Event, EventTx, Role, RoleKind, Team,
};

#[derive(Debug, Clone, Serialize, Deserialize, EnumKind)]
#[enum_kind(PhaseKind, derive(Serialize, Deserialize))]
pub enum Phase {
    Init,
    Day {
        votes: HashMap<u64, Option<u64>>, // voter -> ballot
        blocks: HashMap<u64, Vec<u64>>,   // blocked -> blockers
    },
    Night {
        targets: HashMap<u64, Option<u64>>, // actor -> target
        scheme: Option<(u64, Option<u64>)>, // killer -> mark
    },
    End {
        winning_team: Team,
    },
}

impl std::fmt::Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rules {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum PlayerState {
    Dead,
    #[serde(untagged)]
    Alive(Role),
}

impl From<PlayerState> for Option<Role> {
    fn from(value: PlayerState) -> Self {
        match value {
            PlayerState::Alive(role) => Some(role),
            PlayerState::Dead => None,
        }
    }
}

impl From<Option<Role>> for PlayerState {
    fn from(value: Option<Role>) -> Self {
        match value {
            Some(role) => Self::Alive(role),
            None => Self::Dead,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerLog {
    role: PlayerState, // None means dead
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    log: Vec<(PlayerState, (u32, PhaseKind))>,
}

impl PlayerLog {
    fn from_start_role(role: Role) -> Self {
        Self {
            role: PlayerState::Alive(role),
            log: Vec::new(),
        }
    }
    fn update(&mut self, role: impl Into<PlayerState>, context: (u32, PhaseKind)) {
        self.log.push((self.role, context));
        self.role = role.into();
    }
    fn as_role(&self) -> Option<&Role> {
        match &self.role {
            PlayerState::Alive(role) => Some(role),
            PlayerState::Dead => None,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, EnumKind)]
#[enum_kind(CountKeyKind)]
pub enum CountKey {
    Role(RoleKind),
    Team(Team),
    IsMafia(bool),
    Players,
}

impl std::fmt::Display for CountKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CountKey::Role(rk) => write!(f, "{}", rk),
            CountKey::Team(t) => write!(f, "{}", t),
            CountKey::IsMafia(m) => {
                if *m {
                    write!(f, "Mafia Aligned")
                } else {
                    write!(f, "Not Mafia Aligned")
                }
            }
            CountKey::Players => write!(f, "Players"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct InnerState {
    id: u64,
    day: u32,
    phase: Phase,
    players: HashMap<u64, PlayerLog>,
    rules: Rules,
}

impl InnerState {
    pub fn new<'a>(
        id: u64,
        registry: impl IntoIterator<Item = &'a (u64, Role)>,
        rules: Rules,
    ) -> Self {
        Self {
            id,
            day: 0,
            phase: Phase::Init,
            players: {
                registry
                    .into_iter()
                    .cloned()
                    .map(|(pid, role)| (pid, PlayerLog::from_start_role(role)))
                    .collect()
            },
            rules,
        }
    }

    /// Get iterator over living players
    fn players(&self) -> HashMap<u64, Role> {
        self.players
            .iter()
            .filter_map(|(pid, plog)| Some((*pid, *plog.as_role()?)))
            .collect()
    }

    fn counts(&self, key_kind: CountKeyKind) -> HashMap<CountKey, usize> {
        let mut counts = HashMap::new();
        let count_fn = match key_kind {
            CountKeyKind::Role => |role| CountKey::Role(RoleKind::from(role)),
            CountKeyKind::Team => |role| CountKey::Team(Team::from(role)),
            CountKeyKind::IsMafia => |role| CountKey::IsMafia(Team::from(role) == Team::Mafia),
            CountKeyKind::Players => |_| CountKey::Players,
        };

        for (_, role) in self.players() {
            let key = count_fn(role);
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    pub fn save<W>(&self, writer: W) -> serde_json::Result<()>
    where
        W: Write,
    {
        serde_json::to_writer(writer, self)
    }

    pub fn load<R>(reader: R) -> serde_json::Result<Self>
    where
        R: Read,
    {
        serde_json::from_reader(reader)
    }

    pub fn validate_action(&self, action: &Action) -> Result<(), Error> {
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
                .map(|plog| plog.as_role())
                .flatten()
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
                    .get(&actor)
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

    pub async fn handle_action(&mut self, action: Action, tx: &EventTx) -> Result<()> {
        match action {
            Action::Start => self.handle_start(tx).await?,
            Action::Vote { voter, ballot } => self.handle_vote(voter, ballot, tx).await?,
            _ => todo!(),
        }
        Ok(())
    }

    async fn handle_start(&mut self, tx: &EventTx) -> Result<()> {
        // Roles are already assigned, just start game
        // If number of players is odd, start day, if even, start night
        tx.send(Event::Start {
            id: self.id,
            players: self.players(),
            rules: self.rules.clone(),
            counts: self.counts(CountKeyKind::Team), // TODO: set with rules
        })?;
        if self.players.len() % 2 == 1 {
            self.phase = Phase::Day {
                votes: HashMap::new(),
                blocks: HashMap::new(),
            };
            self.day += 1;
            tx.send(Event::Day {
                day: self.day,
                counts: self.counts(CountKeyKind::Team), // TODO: set with rules
            })?;
        } else {
            self.phase = Phase::Night {
                targets: HashMap::new(),
                scheme: None,
            };
            tx.send(Event::Night {
                day: self.day,
                counts: self.counts(CountKeyKind::Team),
            })?; // TODO: set with rules}
        }
        Ok(())
    }

    async fn handle_vote(
        &mut self,
        voter: u64,
        ballot: Option<Option<u64>>,
        tx: &EventTx,
    ) -> Result<()> {
        let Phase::Day { votes, .. } = &mut self.phase else {
            panic!("Handling vote when phase is not Day");
        };

        let former = if let Some(choice) = ballot {
            votes.insert(voter, choice)
        } else {
            votes.remove(&voter)
        };

        let ballot = ballot.map(|b| (b, votes.values().filter(|c| c == &&b).count()));
        let former = former.map(|f| (f, votes.values().filter(|c| c == &&f).count()));

        tx.send(Event::Vote {
            voter,
            ballot,
            former,
        })?;

        Ok(())
    }

    // async fn check_election(
    //     &mut self,
    //     voter: u64,
    //     ballot: Option<Option<u64>>,
    //     former: Option<Option<u64>>,
    //     tx: &EventTx,
    // ) -> Result<()> {
    //     let Phase::Day { votes, .. } = &mut self.phase else {
    //         panic!("Checking election when phase is not Day");
    //     };

    //     let thresh = self.players.len() / 2 + 1;
    //     let p_thresh = (self.players.len() + 1) / 2;

    //     if let Some(choice) = former {
    //         let choice_count = votes.iter().filter(|(_, c)| (c == &&choice)).count();
    //         tx.send(Event::CheckElection {
    //             choice,
    //             choice_count,
    //         })?;
    //         // Check if we undo an election
    //         let thresh = if choice.is_some() { thresh } else { p_thresh };
    //     }

    //     if let Some(choice) = ballot {
    //         let voters: Vec<u64> = votes
    //             .iter()
    //             .filter_map(|(pid, c)| (c == &choice).then(|| *pid))
    //             .collect();
    //         let choice_count = voters.len();
    //         tx.send(Event::CheckElection {
    //             choice,
    //             choice_count,
    //         })?;
    //         // Check if we cause an election
    //         let thresh = if choice.is_some() { thresh } else { p_thresh };
    //         if choice_count >= thresh {
    //             // Start election timer!
    //             let candidate = choice;
    //             let hammer = voter;
    //             let time = Local::now() + Duration::from_secs(5);
    //             let time_ = time.clone();
    //             let timer_notify = self.timer_notify.clone();
    //             let event = Event::Election {
    //                 candidate,
    //                 hammer,
    //                 voters,
    //             };
    //             let handle = tokio::spawn(async move {
    //                 let mut interval = tokio::time::interval(Duration::from_millis(500));
    //                 while Local::now() < time_ {
    //                     interval.tick().await;
    //                 }
    //                 timer_notify.notify_one();
    //             });
    //             self.timer.replace(Timer {
    //                 time,
    //                 event,
    //                 handle,
    //             });
    //         }
    //     }

    //     Ok(())
    // }

    pub async fn handle_election(
        &mut self,
        candidate: Option<u64>,
        _hammer: u64,
        _voters: Vec<u64>,
        tx: &EventTx,
    ) -> Result<()> {
        // eliminate
        if let Some(pid) = &candidate {
            let plog = self
                .players
                .get_mut(pid)
                .expect("voted player should exist");
            plog.update(PlayerState::Dead, (self.day, PhaseKind::from(&self.phase)));
        }
        // to night
        self.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
        };
        let _ = tx.send(Event::Night {
            day: self.day,
            counts: self.counts(CountKeyKind::Team),
        })?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct State {
    inner: Arc<RwLock<InnerState>>,
    a_tx: ActionTx,
    tx: EventTx,
    handles: Mutex<Option<(JoinHandle<()>, JoinHandle<()>)>>,
}

impl State {
    async fn new(
        inner: InnerState,
        a_tx: ActionTx,
        a_rx: ActionRx,
        tx: EventTx,
    ) -> Result<Arc<Self>> {
        let inner = Arc::new(RwLock::new(inner));
        let state = Arc::new(State {
            inner,
            tx,
            a_tx,
            handles: Mutex::new(None),
        });

        let s1 = state.clone();
        let s2 = state.clone();
        *state.handles.lock().await = Some((
            tokio::spawn(async move { s1.action_handler(a_rx).await }),
            tokio::spawn(async move { s2.event_listener().await }),
        ));
        Ok(state)
    }

    #[tracing::instrument]
    async fn action_handler(&self, mut action_rx: ActionRx) {
        loop {
            match action_rx.recv().await {
                Some((action, responder)) => {
                    debug!(?action);
                    let read_inner = self.inner.read().await;
                    let resp = read_inner.validate_action(&action);
                    drop(read_inner);
                    if let Err(e) = responder.send(resp) {
                        error!(?e);
                    }
                    debug!("Pick up write lock");
                    let mut write_inner = self.inner.write().await;
                    debug!("Got write lock");
                    if let Err(err) = write_inner
                        .handle_action(action, &self.tx)
                        .await
                        .with_context(|| format!("Failed to handle action {:?}", action))
                    {
                        error!(?err);
                    }
                    debug!("handled action");
                }
                None => {
                    debug!("Action channel closed");
                    self.quit().await;
                }
            }
        }
    }

    #[tracing::instrument]
    async fn event_listener(&self) {
        let mut rx = self.tx.subscribe();

        enum State {
            Init,
            Day { n: usize, imm: Option<Election> },
            Night { n: usize },
            DawnImminent,
            End,
        }
        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Event_State Handler?
                    // Can't get voters from last Vote event...
                    // But maybe we can check for election with InnerState
                    // Or we could just always check for election in action handling?
                    // Hmmm then an ElectionImminent event could be used to start the timer
                    // And we can then watch for Votes to see if we need to Avert the Election
                    // So have ElectionImminent and ElectionAverted events.

                    // Then, for Dawn, just have a Dawn Imminent event. Start the timer then go.
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Event channel closed");
                    break;
                }
                Err(err) => {
                    error!(?err);
                }
            }
        }
    }

    async fn quit(&self) {
        if let Some((h1, h2)) = self.handles.lock().await.take() {
            h1.abort();
            h2.abort();
        }
    }
}

pub trait RoleGen {
    fn role_gen(&mut self, n: usize, rng: &mut ThreadRng) -> Result<Vec<Role>>;
}

#[cfg(test)]
mod test {}
