use core::time;
use std::{
    collections::HashMap,
    hash::Hash,
    io::{Read, Write},
    ops::Deref,
    sync::Arc,
    time::Duration,
};

use anyhow::bail;
use chrono::{DateTime, Local};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tokio::{
    sync::{broadcast, Notify},
    task::JoinHandle,
};
use tracing::{event, info};

use super::{interface::EventRx, Action, Error, Event, EventTx, Role, RoleKind, Team};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize)]
struct Timer {
    time: DateTime<Local>,
    event: Event,
    #[serde(skip)]
    handle: JoinHandle<()>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    id: u64,
    day: u32,
    phase: Phase,
    players: HashMap<u64, PlayerLog>,
    rules: Rules,
    #[serde(skip)]
    timer: Option<Timer>,
    #[serde(skip)]
    timer_notify: Arc<Notify>,
}

// TODO: have an event handler that listens to event.
// On votes/targets it checks election!

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
            players: {
                registry
                    .into_iter()
                    .cloned()
                    .map(|(pid, role)| (pid, PlayerLog::from_start_role(role)))
                    .collect()
            },
            rules,
            timer: None,
            timer_notify: Arc::new(Notify::new()),
        }
    }

    /// Get iterator over living players
    fn players(&self) -> HashMap<u64, Role> {
        self.players
            .iter()
            .filter_map(|(pid, plog)| Some((*pid, *plog.as_role()?)))
            .collect()
    }

    fn counts(&self, key_kind: CountKeyKind) -> HashMap<CountKey, u32> {
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

    pub async fn handle_action(
        &mut self,
        action: Action,
        event_tx: &EventTx,
    ) -> anyhow::Result<()> {
        match action {
            Action::Start => self.handle_start(event_tx).await?,
            Action::Vote { voter, ballot } => self.handle_vote(voter, ballot, event_tx).await?,
            _ => todo!(),
        }
        Ok(())
    }

    async fn handle_start(&mut self, event_tx: &EventTx) -> anyhow::Result<()> {
        // Roles are already assigned, just start game
        // If number of players is odd, start day, if even, start night
        event_tx.send(Event::Start {
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
            event_tx.send(Event::Day {
                day: self.day,
                counts: self.counts(CountKeyKind::Team), // TODO: set with rules
            })?;
        } else {
            self.phase = Phase::Night {
                targets: HashMap::new(),
                scheme: None,
            };
            event_tx.send(Event::Night {
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
        event_tx: &EventTx,
    ) -> anyhow::Result<()> {
        let Phase::Day { votes, .. } = &mut self.phase else {
            panic!("Handling vote when phase is not Day");
        };

        let former = if let Some(choice) = ballot {
            votes.insert(voter, choice)
        } else {
            votes.remove(&voter)
        };

        event_tx.send(Event::Vote {
            voter,
            ballot,
            former,
        })?;

        self.check_election(voter, ballot, former, event_tx).await?;

        Ok(())
    }

    async fn check_election(
        &mut self,
        voter: u64,
        ballot: Option<Option<u64>>,
        former: Option<Option<u64>>,
        event_tx: &EventTx,
    ) -> anyhow::Result<()> {
        let Phase::Day { votes, .. } = &mut self.phase else {
            panic!("Checking election when phase is not Day");
        };

        let thresh = self.players.len() / 2 + 1;
        let p_thresh = (self.players.len() + 1) / 2;

        if let Some(choice) = former {
            let choice_count = votes.iter().filter(|(_, c)| (c == &&choice)).count();
            event_tx.send(Event::CheckElection {
                choice,
                choice_count,
            })?;
            // Check if we undo an election
            let thresh = if choice.is_some() { thresh } else { p_thresh };
            self.timer = match self.timer.take() {
                Some(Timer {
                    event: Event::Election { candidate, .. },
                    handle,
                    ..
                }) if choice == candidate && choice_count < thresh => {
                    // Cancel election!!!
                    handle.abort();
                    None
                }
                t => t,
            };
        }

        if let Some(choice) = ballot {
            let voters: Vec<u64> = votes
                .iter()
                .filter_map(|(pid, c)| (c == &choice).then(|| *pid))
                .collect();
            let choice_count = voters.len();
            event_tx.send(Event::CheckElection {
                choice,
                choice_count,
            })?;
            // Check if we cause an election
            let thresh = if choice.is_some() { thresh } else { p_thresh };
            if choice_count >= thresh {
                // Start election timer!
                let candidate = choice;
                let hammer = voter;
                let time = Local::now() + Duration::from_secs(5);
                let time_ = time.clone();
                let timer_notify = self.timer_notify.clone();
                let event = Event::Election {
                    candidate,
                    hammer,
                    voters,
                };
                let handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(500));
                    while Local::now() < time_ {
                        interval.tick().await;
                    }
                    timer_notify.notify_one();
                });
                self.timer.replace(Timer {
                    time,
                    event,
                    handle,
                });
            }
        }

        Ok(())
    }

    pub async fn handle_election(
        &mut self,
        candidate: Option<u64>,
        hammer: u64,
        voters: Vec<u64>,
        event_tx: &EventTx,
    ) -> anyhow::Result<()> {
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
        event_tx.send(Event::Night {
            day: self.day,
            counts: self.counts(CountKeyKind::Team),
        });
        Ok(())
    }

    pub async fn handle_timer(&mut self, event_tx: &EventTx) -> anyhow::Result<()> {
        let timer = self.timer.take();
        let Some(Timer {
            time,
            event,
            handle,
        }) = timer
        else {
            bail!("timer was None while handling timer notified");
        };

        event_tx.send(event.clone());

        match event {
            Event::Election {
                candidate,
                hammer,
                voters,
            } => {
                self.handle_election(candidate, hammer, voters, event_tx)
                    .await?;
            }
            _ => {
                bail!("Got unexpected event while handling timer")
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use tokio::sync::broadcast;
    use tracing::debug;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[test]
    fn serialize_PlayerState() -> anyhow::Result<()> {
        let pstate = PlayerState::Alive(Role::TOWN);
        let dead_pstate = PlayerState::Dead;

        let pstate_ser = serde_json::to_string(&pstate)?;
        debug!(pstate_ser);
        assert!(pstate_ser == "\"TOWN\"");
        let dead_pstate_ser = serde_json::to_string(&dead_pstate)?;
        debug!(dead_pstate_ser);
        assert!(dead_pstate_ser == "\"Dead\"");
        Ok(())
    }

    fn basic_state() -> (State, (EventTx, EventRx)) {
        (
            State::new(
                0,
                &vec![
                    (1, Role::TOWN),
                    (2, Role::COP),
                    (3, Role::DOCTOR),
                    (4, Role::MAFIA),
                ],
                Rules {},
            ),
            broadcast::channel(100),
        )
    }

    fn basic_state_2() -> (State, (EventTx, EventRx)) {
        (
            State::new(
                0,
                &vec![
                    (1, Role::TOWN),
                    (2, Role::TOWN),
                    (3, Role::COP),
                    (4, Role::DOCTOR),
                    (5, Role::MAFIA),
                ],
                Rules {},
            ),
            broadcast::channel(100),
        )
    }

    fn cmp_json<T: Serialize>(actual: &T, expected: serde_json::Value) -> anyhow::Result<bool> {
        let actual = serde_json::from_str::<serde_json::Value>(&serde_json::to_string(actual)?)?;
        let result = expected == actual;
        if !result {
            debug!(name="cmp_json failed", %actual, %expected);
        }
        Ok(result)
    }

    #[tracing::instrument]
    async fn watch(mut rx: EventRx) {
        debug!("Begin watch");
        loop {
            match rx.recv().await {
                Ok(event) => debug!(?event),
                Err(error) => {
                    debug!(?error);
                    if error == broadcast::error::RecvError::Closed {
                        break;
                    }
                }
            }
        }
        debug!("End watch");
    }

    #[traced_test]
    #[test]
    fn serialize_state() -> anyhow::Result<()> {
        let (mut state, (_tx, _rx)) = basic_state();
        let state_ser = serde_json::to_string(&state)?;
        let expected = json!({
            "id": state.id,
            "day": state.day,
            "phase": "Init",
            "players": {
                "1" : {"role": "TOWN"},
                "2" : {"role": "COP"},
                "3" : {"role": "DOCTOR"},
                "4" : {"role": "MAFIA"},
            },
            "rules": {},
        });

        assert!(cmp_json(
            &state,
            json!({
                "id": state.id,
                "day": state.day,
                "phase": "Init",
                "players": {
                    "1" : {"role": "TOWN"},
                    "2" : {"role": "COP"},
                    "3" : {"role": "DOCTOR"},
                    "4" : {"role": "MAFIA"},
                },
                "rules": {},
            })
        )?);

        state
            .players
            .get_mut(&1)
            .map(|plog| plog.update(None, (0, PhaseKind::Day)));

        assert!(cmp_json(
            &state,
            json!({
                "id": state.id,
                "day": state.day,
                "phase": "Init",
                "players": {
                    "1" : {"role": "Dead", "log": [["TOWN", [0, "Day"]]]},
                    "2" : {"role": "COP"},
                    "3" : {"role": "DOCTOR"},
                    "4" : {"role": "MAFIA"},
                },
                "rules": {},
            })
        )?);

        Ok(())
    }

    #[tokio::test]
    async fn start() -> anyhow::Result<()> {
        let (mut state, (tx, mut rx)) = basic_state();
        let action = Action::Start;
        state.handle_action(action, &tx).await?;

        // Test state
        assert!(cmp_json(
            &state,
            json!({
                "id": state.id,
                "day": 0,
                "phase": {"Night": {"targets": {}, "scheme": null}},
                "players" : {
                    "1" : {"role": "TOWN"},
                    "2" : {"role": "COP"},
                    "3" : {"role": "DOCTOR"},
                    "4" : {"role": "MAFIA"},
                },
                "rules":{},
            })
        )?);

        assert!(rx.len() == 2, "Start event and night event");

        assert!(matches!(rx.recv().await?, Event::Start { .. }));
        assert!(matches!(rx.recv().await?, Event::Night { .. }));

        Ok(())
    }

    #[traced_test]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn vote() -> anyhow::Result<()> {
        let (mut s, (tx, mut rx)) = basic_state_2();
        let rx_ = rx.resubscribe();

        let h = tokio::spawn(watch(rx_));

        tokio::time::sleep(Duration::from_secs(1)).await;

        s.handle_action(Action::Start, &tx).await?;

        assert!(rx.len() == 2);
        assert!(matches!(rx.recv().await?, Event::Start { .. }));
        assert!(matches!(rx.recv().await?, Event::Day { .. }));
        let target = Some(Some(2));
        s.handle_action(
            Action::Vote {
                voter: 1,
                ballot: target,
            },
            &tx,
        )
        .await?;

        assert!(rx.len() == 2);
        assert!(matches!(rx.recv().await?, Event::Vote{voter,..} if voter == 1));
        assert!(matches!(rx.recv().await?, Event::CheckElection { .. }));

        s.handle_action(
            Action::Vote {
                voter: 3,
                ballot: target,
            },
            &tx,
        )
        .await?;

        assert!(rx.len() == 2);
        let rx = rx.resubscribe(); // empty the queue

        // Change vote
        s.handle_action(
            Action::Vote {
                voter: 1,
                ballot: Some(Some(4)),
            },
            &tx,
        )
        .await?;

        assert!(rx.len() == 3); // 2 check election events

        // Change vote
        s.handle_action(
            Action::Vote {
                voter: 1,
                ballot: target,
            },
            &tx,
        )
        .await?;

        // Change vote
        s.handle_action(
            Action::Vote {
                voter: 5,
                ballot: Some(Some(2)),
            },
            &tx,
        )
        .await?;

        tokio::time::sleep(Duration::from_secs(6)).await;

        s.handle_timer(&tx).await?;

        drop(tx);

        let _ = h.await;
        Ok(())
    }
}
