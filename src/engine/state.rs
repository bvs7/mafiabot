use night_action::DawnState;
use serde::{de, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, oneshot, RwLock};
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

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct PlayerId(u64);

impl From<PlayerId> for u64 {
    fn from(value: PlayerId) -> Self {
        value.0
    }
}
impl From<u64> for PlayerId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}
impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct GameId(u64);

impl From<GameId> for u64 {
    fn from(value: GameId) -> Self {
        value.0
    }
}
impl From<u64> for GameId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

pub type Choice = Option<PlayerId>;
pub type RawChoice = Option<u64>;
pub type Ballot = Option<Choice>;
pub type RawBallot = Option<RawChoice>;

fn new_event_tx() -> EventTx {
    broadcast::Sender::new(100)
}

// TODO: Figure out a way to pass in a "GameImpl"? To make some things more hidden from state:
// Action recv
// Event send
// Timers

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    id: GameId,
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    #[serde(skip, default = "new_event_tx")]
    event_tx: EventTx,
    // Want to be able to start timers
    // Want to be able to respond to events
}

impl State {
    pub fn new(id: GameId, registry: Vec<(u64, Role)>, rules: Rules) -> Self {
        let event_tx = new_event_tx();
        Self {
            id,
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_tx,
        }
    }

    pub fn game_id(&self) -> GameId {
        self.id
    }

    #[instrument(skip_all)]
    pub async fn action_handler(this: Arc<RwLock<Self>>, action_rx: &mut ActionRx) {
        while let Some((action, resp)) = action_rx.recv().await {
            let mut wstate = this.write().await;
            wstate.handle_action(action, resp, &this).await;
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

    // The game pointer is used for... Event tx, to send events...
    // And for timers to have a handle on the state mutex...
    // It would be nice to do this differently...
    pub async fn handle_action(
        &mut self,
        action: Action,
        resp: oneshot::Sender<Result<(), Error>>,
        this: &Arc<RwLock<Self>>,
    ) {
        let result = match action {
            Action::Start => self.start(),
            Action::Vote { voter, ballot } => self.vote(voter, ballot, this),
            Action::Reveal { actor } => self.reveal(actor),
            Action::Target { actor, choice } => self.target(actor, choice, this),
            Action::Scheme { killer, mark } => self.scheme(killer, mark, this),
        };
        let _ = resp.send(result);
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let Phase::Init = self.phase else {
            return Err(Error::InvalidPhase {
                expected: PhaseKind::Init,
                actual: self.phase.kind(),
            });
        };
        let _ = self.event_tx.send(Event::Start {
            id: self.id,
            players: self.players.alive(),
            rules: self.rules.clone(),
            counts: self.counts(Team::from), // Use rules for this
        });
        if self.players.alive().len() % 2 == 1 {
            self.to_day(HashMap::new());
        } else {
            self.to_night();
        }
        Ok(())
    }

    pub fn vote(
        &mut self,
        voter: u64,
        ballot: RawBallot,
        this: &Arc<RwLock<Self>>,
    ) -> Result<(), Error> {
        let voter = self.players.validate(voter)?;
        let ballot = self.players.validate_ballot(ballot)?;
        let former = self.phase.vote(voter, ballot)?;
        let vote_list = self.phase.vote_list()?;

        let ballot = ballot.map(|c| (c, vote_list.get(&c).unwrap().len()));
        let former = former.map(|c| (c, vote_list.get(&c).unwrap().len()));

        self.tx(Event::Vote {
            voter,
            ballot: ballot.clone(),
            former: former.clone(),
        });

        self.check_election(voter, this);
        Ok(())
    }

    fn reveal(&self, actor: u64) -> Result<(), Error> {
        let actor = self.players.validate(actor)?;
        let role = self.players.get(actor);
        if role != Role::CELEB {
            return Err(Error::ExpectedCeleb {
                actual: role.kind(),
            });
        }
        let Phase::Day { blocks, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Day);
        };
        if let Some(blockers) = blocks.get(&actor) {
            self.tx(Event::Block {
                blocked: actor,
                blockers: blockers.clone(),
            });
        } else {
            self.tx(Event::Reveal { celeb: actor });
        }
        Ok(())
    }

    fn target(
        // TODO: Stripper must pick one of target/scheme
        &mut self,
        actor: u64,
        choice: RawChoice,
        this: &Arc<RwLock<Self>>,
    ) -> Result<(), Error> {
        let actor = self.players.validate(actor)?;
        let choice = self.players.validate_choice(choice)?;
        self.phase.target(actor, choice)?;
        self.check_dawn(this);
        Ok(())
    }

    fn scheme(
        // TODO: Stripper must pick one of target/scheme
        &mut self,
        killer: u64,
        mark: RawChoice,
        this: &Arc<RwLock<Self>>,
    ) -> Result<(), Error> {
        let killer = self.players.validate(killer)?;
        let mark = self.players.validate_choice(mark)?;
        self.phase.scheme(killer, mark)?;
        self.check_dawn(this);
        Ok(())
    }
    fn check_election(&mut self, hammer: PlayerId, this: &Arc<RwLock<Self>>) {
        debug!("Check election");
        let Ok(vote_list) = self.phase.vote_list() else {
            return;
        };
        debug!("Vote list: {:?}", vote_list);
        let n = self.players.alive().len();
        let pthresh = (n + 1) / 2;
        let thresh = (n / 2) + 1;
        debug!(?n, ?thresh, ?pthresh);
        let mut new_elect = None;
        for (key, value) in vote_list {
            let t = if key.is_some() { thresh } else { pthresh };
            if value.len() >= t {
                new_elect = Some((key, value));
            }
        }
        debug!("New elect {:?}", new_elect);
        let Phase::Day { pend_elect, .. } = &mut self.phase else {
            return;
        };
        // Check if an election is cancelled
        if let Some((old_choice, h)) = pend_elect {
            if new_elect
                .as_ref()
                .is_none_or(|(new_choice, _)| new_choice != old_choice)
            {
                h.abort()
            }
        }
        // Check if a new election is started
        if pend_elect.is_none() {
            if let Some((choice, _)) = new_elect {
                let h = tokio::spawn(Self::election_timer(this.clone(), choice, hammer));
                *pend_elect = Some((choice, h.abort_handle()));
            }
        }
    }

    async fn election_timer(this: Arc<RwLock<Self>>, choice: Choice, hammer: PlayerId) {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let mut wstate = this.write().await;
        if let Phase::Day {
            pend_elect: Some((pend_choice, _)),
            ..
        } = &wstate.phase
        {
            if pend_choice == &choice {
                let voters = wstate
                    .phase
                    .vote_list()
                    .unwrap_or_default()
                    .remove(pend_choice)
                    .unwrap_or_default();

                wstate.election(choice, hammer, voters);
            }
        }
    }

    fn election(&mut self, choice: Choice, hammer: PlayerId, voters: Vec<PlayerId>) {
        self.tx(Event::Election {
            choice,
            hammer,
            voters: voters.clone(),
        });
        if let Some(pid) = choice {
            // TODO: Check if IDIOT (And add eclipse phase?)
            self.eliminate(pid, hammer, Context::new(self.day, Cause::Election));
        }
        self.to_night();
    }

    fn check_dawn(&mut self, this: &Arc<RwLock<Self>>) {
        let Phase::Night {
            targets,
            scheme,
            pend_dawn,
        } = &mut self.phase
        else {
            return;
        };
        if pend_dawn.is_some() {
            return;
        }
        if scheme.is_none() {
            return;
        }
        for (pid, role) in self.players.alive() {
            if role.is_targeting() && !targets.contains_key(&pid) {
                return;
            }
        }
        let h = tokio::spawn(Self::dawn_timer(this.clone()));
        *pend_dawn = Some(h.abort_handle());
    }

    async fn dawn_timer(this: Arc<RwLock<Self>>) {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let mut wstate = this.write().await;
        wstate.dawn();
    }

    fn dawn(&mut self) {
        debug!("Dawn");
        self.tx(Event::Dawn);
        let night_actions = self.phase.to_night_actions(&self.players);
        let ds = night_actions.fold(DawnState::default(), |acc, na| acc.fold(na, &self));
        for (mark, killer) in ds.kills {
            self.tx(Event::Kill { killer, mark });
            self.eliminate(mark, killer, Context::new(self.day, Cause::Kill));
        }

        self.to_day(ds.blocks);
    }

    fn eliminate(&mut self, pid: PlayerId, _culpable: PlayerId, context: Context) {
        debug!("Eliminate {}", pid);
        // TODO: Check if a charge
        let PlayerState::Alive(role) = self.players.eliminate(&pid, context) else {
            panic!("Eliminating a dead player?");
        };
        self.tx(Event::Eliminate {
            player: pid,
            role: role.kind(),
            context,
        });
    }

    fn to_day(&mut self, blocks: Blocks) {
        debug!("To day");
        if self.check_win() {
            return;
        }
        self.day += 1;
        self.phase = Phase::Day {
            votes: HashMap::new(),
            blocks,
            pend_elect: None,
        };
        self.tx(Event::Day {
            day: self.day,
            counts: self.counts(Team::from), // TODO: use rules
        })
    }

    fn to_night(&mut self) {
        debug!("To night");
        if self.check_win() {
            return;
        }
        self.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };
        self.tx(Event::Night {
            day: self.day,
            counts: self.counts(Team::from), // TODO: use rules
        })
    }

    fn check_win(&mut self) -> bool {
        debug!("Check win");
        let n = self.players.alive().len();
        let n_maf = self
            .players
            .alive()
            .iter()
            .filter(|(_, r)| r.is_mafia())
            .count();
        let mut winner = None;
        if n_maf == 0 {
            debug!("Town wins");
            winner = Some(Team::Town);
        } else if n - n_maf <= n_maf {
            debug!("Mafia wins");
            winner = Some(Team::Mafia);
        }
        if let Some(winner) = winner {
            self.phase = Phase::End { winner };
        }
        return winner.is_some();
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

#[cfg(test)]
mod tests {
    use core::time;

    use super::*;

    // We want to test State specific functionality here, not necessarily full games, right?

    fn basic_game() -> State {
        let registry = vec![
            (1, Role::TOWN),
            (2, Role::COP),
            (3, Role::DOCTOR),
            (4, Role::MAFIA),
        ];
        let rules = Rules::default();
        State::new(GameId(1), registry, rules)
    }

    #[tracing::instrument(skip_all)]
    async fn listen_events(mut rx: EventRx) {
        while let Ok(event) = rx.recv().await {
            info!("Event: {:?}", event);
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn basic_test() {
        let mut state = basic_game();
        let event_rx = state.subscribe();
        tokio::spawn(listen_events(event_rx));
        let this = Arc::new(RwLock::new(state));
        let mut wstate = this.write().await;
        wstate.start().expect("State starts in Init");
        assert!(matches!(wstate.phase, Phase::Night { .. }));
        wstate
            .target(2, Some(3), &this)
            .expect("Cop can target at night");
        wstate
            .target(3, Some(3), &this)
            .expect("Doctor can target at night");
        wstate
            .scheme(4, Some(1), &this)
            .expect("Mafia can scheme at night");
        assert!(matches!(wstate.phase, Phase::Night { .. }));
        drop(wstate);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let mut wstate = this.write().await;
        wstate.dawn();
        assert!(matches!(wstate.phase, Phase::Day { .. }));
        assert_eq!(wstate.day, 1);
        assert_eq!(wstate.players.alive().len(), 3);

        wstate
            .vote(2, Some(Some(4)), &this)
            .expect("Can vote during Day");
        wstate
            .vote(3, Some(Some(4)), &this)
            .expect("Can vote during Day");
        assert!(matches!(wstate.phase, Phase::Day { .. }));
        drop(wstate);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let mut wstate = this.write().await;
        wstate.election(Some(4.into()), 3.into(), vec![2.into(), 3.into()]);
        assert!(matches!(&wstate.phase, Phase::End { winner: Team::Town }));
    }

    #[tokio::test]
    async fn basic_test_errs() {
        let mut state = basic_game();
        let event_rx = state.subscribe();
        tokio::spawn(listen_events(event_rx));
        let this = Arc::new(RwLock::new(state));
        let mut wstate = this.write().await;
        assert!(wstate.vote(2, Some(Some(4)), &this).is_err());
        assert!(wstate.reveal(2).is_err());
        assert!(wstate.target(2, Some(3), &this).is_err());
        assert!(wstate.scheme(4, Some(3), &this).is_err());
        // Actually start
        wstate.start().expect("State starts in Init");
        assert!(wstate.start().is_err());
        assert!(wstate.vote(2, Some(Some(4)), &this).is_err());
        assert!(wstate.reveal(2).is_err());
    }
}
