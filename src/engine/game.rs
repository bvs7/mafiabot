mod base;
pub use base::*;

mod state;
pub use state::{PhaseKind, State, Status};

mod interface;
use interface::*;
pub mod rules;
pub use rules::Rules;

use chrono::{DateTime, Local, OutOfRangeError};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, oneshot, Mutex, RwLock, TryLockError},
    time::timeout,
};

enum NoAction {
    Timeout,
    Closed,
}

impl From<OutOfRangeError> for NoAction {
    fn from(_: OutOfRangeError) -> Self {
        NoAction::Timeout
    }
}
impl From<tokio::time::error::Elapsed> for NoAction {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        NoAction::Timeout
    }
}

pub struct Game {
    state: RwLock<State>,
    action_rx: Mutex<ActionRx>,
    event_tx: EventTx,
}

impl Game {
    pub fn new(mut state: State) -> (Self, ActionTx) {
        let (action_tx, action_rx) = tokio::sync::mpsc::channel(100);
        let (event_tx, _) = tokio::sync::broadcast::channel(100);
        state.event_tx = Some(event_tx.clone());
        (Self { state: RwLock::new(state), action_rx: Mutex::new(action_rx), event_tx }, action_tx)
    }

    pub fn event_rx(&self) -> EventRx {
        self.event_tx.subscribe()
    }

    pub async fn status(&self, names: HashMap<impl Into<Pid>, String>) -> Status {
        let state = self.state.read().await;
        state.status(names)
    }

    pub fn start(self) -> Arc<Self> {
        let game = Arc::new(self);
        let g = game.clone();
        tokio::spawn(async move { g.run().await });
        game
    }

    /// Run the game loop, which involves waiting for actions or for timers to expire,
    /// handling the actions, and updating the game state.
    pub async fn run(&self) -> Result<(), TryLockError> {
        let mut action_rx = self.action_rx.try_lock()?;
        let mut alarm_time = None;
        loop {
            match self.next_action(alarm_time, &mut action_rx).await {
                Ok((action, resp)) => {
                    self.action(action, resp).await;
                }
                Err(NoAction::Timeout) => {}
                Err(NoAction::Closed) => break,
            }
            alarm_time = self.update().await;
        }
        Ok(())
    }

    async fn next_action(
        &self,
        alarm_time: Option<DateTime<Local>>,
        action_rx: &mut ActionRx,
    ) -> Result<ActionMsg, NoAction> {
        let dur = match alarm_time {
            Some(time) => (time - Local::now()).to_std()?,
            None => Duration::MAX,
        };
        let action = timeout(dur, action_rx.recv()).await?;
        match action {
            Some((action, resp)) => Ok((action, resp)),
            None => Err(NoAction::Closed),
        }
    }

    /// First check action with read lock. If action is invalid, return error.
    /// Then check action again with write lock, to be sure it is still valid.
    /// If action is still valid, send ok then perform action.
    async fn action(&self, action: Action, resp: ActionResponder) {
        let rstate = self.state.read().await;
        if let Err(err) = rstate.validate_action(&action) {
            let _ = resp.send(Err(err));
            return;
        }
        drop(rstate);
        let mut wstate = self.state.write().await;
        let action = match wstate.validate_action(&action) {
            Ok(action) => action,
            Err(err) => {
                let _ = resp.send(Err(err));
                return;
            }
        };
        let _ = resp.send(Ok(()));
        wstate.perform_action(action);
        wstate.update();
    }

    async fn update(&self) -> Option<DateTime<Local>> {
        let mut wstate = self.state.write().await;
        wstate.update()
    }
}
/*
#[cfg(test)]
mod tests {
    use core::panic;
    use std::f32::consts::E;

    use tracing_test::traced_test;

    use super::*;
    fn pid(n: u64) -> Pid {
        Pid::from(n)
    }

    fn state_3() -> State {
        State {
            day: 1,
            phase: Phase::Day { votes: HashMap::new(), blocks: HashMap::new(), elect: None },
            players: Players::from_registry(vec![
                (1, Role::TOWN),
                (2, Role::TOWN),
                (3, Role::MAFIA),
            ]),
            rules: Rules::default(),
            event_log: Vec::new(),
            event_tx: tokio::sync::broadcast::channel(100).0,
        }
    }

    #[test]
    fn vote_pass() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);

        let mut state = state_3();

        state.vote(1, Some(Some(1))).expect("Vote self should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(one));
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, None).expect("Unvote should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one), None);
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, Some(Some(2))).expect("Vote other should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(two));
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, Some(Some(3))).expect("Vote again should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(three));
        } else {
            panic!("Expected Day phase");
        }

        state.vote(1, Some(None)).expect("Vote none should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &None);
        } else {
            panic!("Expected Day phase");
        }
    }

    #[test]
    fn vote_fail() {
        // Votes in wrong phase
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let mut state = state_3();

        let err = state.vote(1, Some(Some(2))).expect_err("Vote in init should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };

        let err = state.vote(1, Some(Some(3))).expect_err("Vote in night should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::End { winner: Team::Town };

        let err = state.vote(1, Some(None)).expect_err("Vote in end should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day { votes: HashMap::new(), blocks: HashMap::new(), elect: None };

        // Invalid voter
        let err = state.vote(4, Some(Some(2))).expect_err("Invalid voter should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Invalid ballot

        let err = state.vote(1, Some(Some(4))).expect_err("Invalid ballot should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Ineffective vote

        state.vote(1, Some(Some(2))).expect("Vote should work");
        let err = state.vote(1, Some(Some(2))).expect_err("Ineffective vote should fail");

        assert!(matches!(err, Error::IneffectiveAction));
    }

    #[test]
    fn elect_update() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let mut state = state_3();

        state.vote(1, Some(Some(3))).expect("Vote self should work");
        state.vote(2, Some(Some(3))).expect("Vote self should work");

        // Now, after one update, election should be scheduled
        state.update();
        if let Phase::Day { elect: pend_elect, .. } = &mut state.phase {
            assert!(pend_elect.is_some());
            *pend_elect =
                Some((Some(Pid::from(3)), Pid::from(2), Local::now() - Duration::from_secs(1)));
        } else {
            panic!("Expected Day phase");
        }
        // After another update, with time changed, election should be done
        state.update();

        if let Phase::End { winner } = &state.phase {
            assert_eq!(winner, &Team::Town);
        } else {
            panic!("Expected End phase");
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn target_pass() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let four = Pid::from(4);
        let mut state = start_state_6();
        state.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };

        state.target(2, Some(4)).expect("Target self should work");
        if let Phase::Night { targets, .. } = &state.phase {
            assert_eq!(targets.get(&two).unwrap(), &Some(four));
        } else {
            panic!("Expected Night phase");
        }

        state.target(2, Some(3)).expect("Change target should work");
        if let Phase::Night { targets, .. } = &state.phase {
            assert_eq!(targets.get(&two).unwrap(), &Some(three));
        } else {
            panic!("Expected Night phase");
        }

        state.target(2, None).expect("Change to None Target should work");
        if let Phase::Night { targets, .. } = &state.phase {
            assert_eq!(targets.get(&two).unwrap(), &None);
        } else {
            panic!("Expected Night phase");
        }

        state.target(3, Some(2)).expect("Target other should work");

        state.scheme(4, Some(1)).expect("Scheme should work");

        state.target(5, Some(6)).expect("Stripper should work");
        state.target(6, Some(2)).expect("Milk should work");

        state.update(); // Dawn should be scheduled

        state.target(2, Some(4)).expect("Target should work even with dawn pending");

        if let Phase::Night { dawn, .. } = &mut state.phase {
            assert!(pend_dawn.is_some());
        } else {
            panic!("Expected Night phase");
        }
    }

    #[test]
    fn target_fail() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let four = Pid::from(4);
        let mut state = start_state_6();
        state.phase = Phase::Night { targets: HashMap::new(), scheme: None, dawn: None };

        let err = state.target(7, Some(1)).expect_err("Invalid actor should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        let err = state.target(2, Some(7)).expect_err("Invalid target should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        let err = state.target(1, Some(4)).expect_err("Invalid role should fail");

        assert!(matches!(err, Error::ExpectedTargetingRole { actual: RoleKind::TOWN }));

        state.phase = Phase::Init;

        let err = state.target(2, Some(4)).expect_err("Target in init should fail");

        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day { votes: HashMap::new(), blocks: HashMap::new(), elect: None };

        let err = state.target(2, Some(4)).expect_err("Target in day should fail");

        assert!(matches!(err, Error::InvalidPhase { actual: PhaseKind::Day, .. }));
    }

    fn start_state_6() -> State {
        State::new(
            vec![
                (1, Role::TOWN),
                (2, Role::COP),
                (3, Role::DOCTOR),
                (4, Role::MAFIA),
                (5, Role::STRIPPER),
                (6, Role::MILKY),
            ],
            Rules::default(),
        )
    }

    #[test]
    fn test_dawn1() {
        let start_state = start_state_6();
        // successful save, block cop
        let mut state1 = start_state.clone();
        state1.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(4))),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(2))),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(3)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state1.update();

        assert!(matches!(state1.phase, Phase::Day { .. }));
        assert!(matches!(state1.players.n(), 6));

        let events = &state1.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Dawn,
            Event::Block { blocked: pid(2), blockers: vec![pid(5)] },
            Event::Save { saved: pid(3), saviors: vec![pid(3)] },
            Event::Milk { milky: pid(6), target: pid(1) },
            Event::NoKill,
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 4), (Team::Mafia, 2)].into_iter().collect(),
            },
        ];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn2() {
        let mut state = start_state_6();
        // no save, investigation and milked killed
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(4))),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(1))),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(1)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Investigate { cop: pid(2), target: pid(4), appears_mafia: true },
            Event::Kill { killer: pid(4), mark: pid(1) },
        ];
        let exp_not_events = vec![Event::Kill { killer: pid(4), mark: pid(3) }];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn3() {
        let mut state = start_state_6();
        // block save and kill doc
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), None),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(3))),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(3)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Kill { killer: pid(4), mark: pid(3) },
            Event::Block { blocked: pid(3), blockers: vec![pid(5)] },
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 3), (Team::Mafia, 2)].into_iter().collect(),
            },
            Event::Milk { milky: pid(6), target: pid(1) },
        ];
        let exp_not_events = vec![Event::NoKill];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn4() {
        let mut state = start_state_6();
        // cop killed, ineffective save blocked
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(1))),
                (pid(3), Some(pid(3))),
                (pid(5), Some(pid(3))),
                (pid(6), None),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(2)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![Event::Kill { killer: pid(4), mark: pid(2) }];
        let exp_not_events = vec![
            Event::Investigate { cop: pid(2), target: pid(1), appears_mafia: false },
            Event::Block { blocked: pid(3), blockers: vec![pid(5)] },
        ];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    #[test]
    #[traced_test]
    fn test_dawn5() {
        let mut state = start_state_6();
        // posthumous milk, dead investigated target
        state.phase = Phase::Night {
            targets: vec![
                (pid(2), Some(pid(6))),
                (pid(3), Some(pid(2))),
                (pid(5), None),
                (pid(6), Some(pid(1))),
            ]
            .into_iter()
            .collect(),
            scheme: Some((pid(4), Some(pid(6)))),
            dawn: Some(Local::now() - Duration::from_secs(1)),
        };

        state.update();

        assert!(matches!(state.phase, Phase::Day { .. }));

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Kill { killer: pid(4), mark: pid(6) },
            Event::Day {
                day: 1,
                counts: vec![(Team::Town, 3), (Team::Mafia, 2)].into_iter().collect(),
            },
            Event::Milk { milky: pid(6), target: pid(1) },
            Event::Investigate { cop: pid(2), target: pid(6), appears_mafia: false },
        ];
        let exp_not_events = vec![];

        for ee in exp_events {
            assert!(events.contains(&ee), "Could not find event: {:?}", ee);
        }
        for nee in exp_not_events {
            assert!(!events.contains(&nee), "Found bad event: {:?}", nee);
        }
    }

    fn start_state_7() -> State {
        State::new(
            vec![
                (1, Role::MILLER),
                (2, Role::COP),
                (3, Role::DOCTOR),
                (4, Role::CELEB),
                (5, Role::IDIOT),
                (6, Role::GODFATHER),
                (7, Role::STRIPPER),
            ],
            Rules::default(),
        )
    }

    #[test]
    #[traced_test]
    fn test_eclipse_basic() {
        let mut state = start_state_7();

        state.phase = Phase::Day {
            votes: vec![(pid(1), Some(pid(5))), (pid(2), Some(pid(5))), (pid(3), Some(pid(5)))]
                .into_iter()
                .collect(),
            blocks: HashMap::new(),
            elect: None,
        };

        state.vote(5, Some(Some(5))).expect("Vote self should work");

        state.update();

        if let Phase::Day { elect, .. } = &mut state.phase {
            let Some((choice, hammer, time)) = &elect else {
                return panic!("Expected pending election");
            };
            let new_elect = Some((*choice, *hammer, *time - Duration::from_secs(100)));
            *elect = new_elect;
        } else {
            panic!("Expected Day phase");
        }

        state.update();

        if let Phase::Eclipse { avenger, hammer, guilty, vote } = &state.phase {
            assert_eq!(avenger, &pid(5));
            assert_eq!(hammer, &pid(5));
            for v in vec![pid(1), pid(2), pid(3), pid(5)] {
                assert!(guilty.contains(&v));
            }
            assert_eq!(vote, &None);
        } else {
            panic!("Expected Eclipse phase");
        }

        // Check fail votes during eclipse
        let err =
            state.vote(1, Some(Some(2))).expect_err("Voter not avenger during eclipse should fail");
        assert!(matches!(err, Error::IneffectiveAction));

        let err =
            state.vote(5, Some(Some(6))).expect_err("Avenger voting for not guilty should fail");
        assert!(matches!(err, Error::IneffectiveAction));

        let err = state.vote(5, None).expect_err("Avenger unvoting should fail");
        assert!(matches!(err, Error::IneffectiveAction));

        let err = state.vote(5, Some(None)).expect_err("Avenger voting for peace should fail");

        let err = state.vote(5, Some(Some(5))).expect_err("Avenger voting for self should fail");

        state.vote(5, Some(Some(1))).expect("Avenger voting for guilty should work");

        state.update();

        assert!(matches!(state.phase, Phase::Night { .. }));
        assert_eq!(state.players.n(), 5);

        let events = &state.event_log[0..];
        tracing::debug!("Events: {:#?}", events);
        let exp_events = vec![
            Event::Election {
                choice: Some(pid(1)),
                hammer: pid(5),
                voters: vec![pid(1), pid(2), pid(3), pid(5)],
            },
            Event::Eclipse {
                avenger: pid(5),
                hammer: pid(5),
                guilty: vec![pid(1), pid(2), pid(3), pid(5)],
            },
            Event::Vengeance { avenger: pid(5), victim: pid(1) },
            Event::Eliminate {
                player: pid(1),
                role: RoleKind::MILLER,
                context: Context::new(0, Cause::Vengeance),
            },
            Event::Eliminate {
                player: pid(5),
                role: RoleKind::IDIOT,
                context: Context::new(0, Cause::Election),
            },
        ];
    }
}
*/
