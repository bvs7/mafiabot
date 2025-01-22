use core::time;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Condvar, Mutex, MutexGuard, TryLockError},
    thread::{park_timeout, JoinHandle, Thread},
    time::Duration,
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use toml::value::Date;
use tracing::error;

use super::{
    id::{Choice, Gid, Pid, RawBallot, RawChoice},
    phase::{Blocks, Phase, PhaseKind, Votes},
    players::{Cause, Context, PlayerState, Players},
    Error, Event, Role, RoleKind, Rules, Team,
};

const ELECTION_DELAY: Duration = Duration::from_secs(10);
const DAWN_DELAY: Duration = Duration::from_secs(10);
const TICK_DELAY: Duration = Duration::from_secs(1);
/*
Have scheduled events? Or at least a thread that will notify when needed.
*/

fn count_votes(votes: &Votes) -> HashMap<Choice, Vec<Pid>> {
    let mut vote_list: HashMap<Choice, Vec<Pid>> = HashMap::new();
    for (voter, choice) in votes {
        vote_list.entry(*choice).or_default().push(*voter);
    }
    vote_list
}

fn thresh(n: usize, choice: &Choice) -> usize {
    if choice.is_some() {
        return (n / 2) + 1;
    } else {
        return (n + 1) / 2;
    }
}

fn last_vote_for(event_log: &Vec<Event>, choice: &Choice) -> Option<Pid> {
    for event in event_log.iter().rev() {
        match event {
            Event::Vote {
                voter,
                ballot: Some((c, _)),
                ..
            } => {
                if c == choice {
                    return Some(*voter);
                }
            }
            _ => {}
        }
    }
    None
}

enum UpdateResult {
    Election(Choice, Pid, Vec<Pid>),
    Dawn,
}

// How else could we implement update_flag?
//

pub struct State_ {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    event_log: Vec<Event>,
    update_thread: Option<JoinHandle<()>>,
}

impl State_ {
    pub fn new(registry: impl IntoIterator<Item = (u64, Role)>, rules: Rules) -> Self {
        Self {
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_log: Vec::new(),
            update_thread: None,
        }
    }

    pub fn wake(&self) {
        if let Some(handle) = &self.update_thread {
            handle.thread().unpark();
        }
    }

    fn log_event(&mut self, event: Event) {
        self.event_log.push(event);
    }

    pub fn events_from(&self, from: usize) -> Vec<Event> {
        self.event_log[from..].to_vec()
    }

    pub fn start(&mut self) {
        self.log_event(Event::Start {
            players: self.players.alive(),
            rules: self.rules.clone(),
        });
        let n = self.players.n();
        if n % 2 == 1 {
            self.day(HashMap::new());
        } else {
            self.night();
        }
        self.wake();
    }
    pub fn vote(&mut self, voter: u64, ballot: RawBallot) -> Result<(), Error> {
        let voter = self.players.validate(voter)?;
        let ballot = self.players.validate_ballot(ballot)?;
        let former = self.phase.vote(voter, ballot)?;
        let mut vote_list = self.phase.vote_list()?;

        let ballot = ballot.map(|c| (c, vote_list.entry(c).or_default().len()));
        let former = former.map(|c| (c, vote_list.entry(c).or_default().len()));

        self.log_event(Event::Vote {
            voter,
            ballot,
            former,
        });
        self.wake();
        Ok(())
    }

    pub fn reveal(&mut self, celeb: u64) -> Result<(), Error> {
        let celeb = self.players.validate(celeb)?;
        let role = self.players.get(celeb);
        if role != Role::CELEB {
            return Err(Error::ExpectedCeleb {
                actual: role.kind(),
            });
        }
        let Phase::Day { blocks, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Day);
        };
        if let Some(blockers) = blocks.get(&celeb) {
            self.log_event(Event::Block {
                blocked: celeb,
                blockers: blockers.clone(),
            });
        } else {
            // Check if celeb is blocked
            self.log_event(Event::Reveal { celeb });
        }
        Ok(())
    }

    pub fn target(&mut self, actor: u64, choice: RawChoice) -> Result<(), Error> {
        let actor = self.players.validate(actor)?;
        let choice = self.players.validate_choice(choice)?;
        let role = self.players.get(actor);
        if !role.is_targeting() {
            return Err(Error::ExpectedTargetingRole {
                actual: role.kind(),
            });
        }
        self.log_event(Event::Target { actor, choice });
        self.wake();
        Ok(())
    }

    pub fn scheme(&mut self, killer: u64, mark: RawChoice) -> Result<(), Error> {
        let killer = self.players.validate(killer)?;
        let mark = self.players.validate_choice(mark)?;
        let role = self.players.get(killer);
        if !role.is_scheming() && mark.is_some() {
            return Err(Error::ExpectedSchemingRole {
                actual: role.kind(),
            });
        }
        self.log_event(Event::Scheme { killer, mark });
        self.wake();
        Ok(())
    }

    pub fn elect(&mut self, choice: Choice, hammer: Pid, voters: Vec<Pid>) -> Result<(), Error> {
        self.log_event(Event::Election {
            choice,
            hammer,
            voters,
        });
        if let Some(pid) = choice {
            self.eliminate(pid, hammer, Context::new(self.day, Cause::Election));
        }
        Ok(())
    }

    pub fn dawn(&mut self) -> Blocks {
        return HashMap::new();
    }

    pub fn eliminate(&mut self, player: Pid, _culpable: Pid, context: Context) {
        let PlayerState::Alive(role) = self.players.eliminate(&player, context) else {
            panic!("Eliminating a dead player?");
        };
        let role = role.kind();
        self.log_event(Event::Eliminate {
            player,
            role,
            context,
        });
    }

    pub fn day(&mut self, blocks: Blocks) {
        self.day += 1;
        self.phase = Phase::Day {
            votes: HashMap::new(),
            blocks,
            pend_elect: None,
        };
        let counts = self.players.counts(Team::from);
        self.log_event(Event::Day {
            day: self.day,
            counts,
        });
    }

    pub fn night(&mut self) {
        self.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };
        let counts = self.players.counts(Team::from);
        self.log_event(Event::Night {
            day: self.day,
            counts,
        });
    }

    pub fn check_end(&mut self) -> bool {
        let counts = self.players.counts(Team::from);
        let n = self.players.alive().len();
        let n_maf = counts.get(&Team::Mafia).copied().unwrap_or(0);
        if n_maf == 0 {
            self.phase = Phase::End { winner: Team::Town };
            return true;
        } else if n - n_maf <= n_maf {
            self.phase = Phase::End {
                winner: Team::Mafia,
            };
            return true;
        }
        false
    }

    /// Test for an update in phase, or for phase ending updates
    fn poll_update(&mut self) -> Option<UpdateResult> {
        match &mut self.phase {
            Phase::Day {
                votes, pend_elect, ..
            } => {
                let mut vote_list = count_votes(&votes);
                let n = self.players.n();
                if let Some((choice, hammer, time)) = pend_elect {
                    let voters = vote_list.entry(*choice).or_default();
                    let thresh = thresh(n, choice);
                    // Check for averted election
                    if voters.len() < thresh {
                        *pend_elect = None;
                    } else if *time < Local::now() {
                        return Some(UpdateResult::Election(*choice, *hammer, voters.clone()));
                    }
                }
                if pend_elect.is_none() {
                    for (choice, voters) in vote_list {
                        let thresh = thresh(n, &choice);
                        if voters.len() >= thresh {
                            let hammer = last_vote_for(&self.event_log, &choice).unwrap();
                            let time = Local::now() + ELECTION_DELAY;
                            *pend_elect = Some((choice, hammer, time));
                        }
                    }
                }
                None
            }
            Phase::Night {
                targets,
                scheme,
                pend_dawn,
            } => {
                if let Some(time) = pend_dawn {
                    if *time < Local::now() {
                        return Some(UpdateResult::Dawn);
                    }
                } else {
                    let mut ready = true;
                    if scheme.is_none() {
                        ready = false;
                    }
                    for (pid, role) in self.players.alive() {
                        if targets.get(&pid).is_none() {
                            ready = false;
                        }
                    }
                    if ready {
                        let time = Local::now() + DAWN_DELAY;
                        *pend_dawn = Some(time);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn handle_update(&mut self, result: UpdateResult) {
        match result {
            UpdateResult::Election(choice, hammer, voters) => {
                self.elect(choice, hammer, voters);
                if !self.check_end() {
                    self.night();
                }
            }
            UpdateResult::Dawn => {
                let blocks = self.dawn();
                if !self.check_end() {
                    self.day(blocks);
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    fn run(state: Arc<Mutex<Self>>) {
        loop {
            park_timeout(TICK_DELAY);
            let mut lock = match state.lock() {
                Ok(s) => s,
                Err(err) => {
                    error!("Failed to lock state: {:?}", err);
                    return;
                }
            };

            if let Some(result) = lock.poll_update() {
                lock.handle_update(result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::panic;

    use super::*;

    #[test]
    fn vote_pass() {
        let one = Pid::from(1);
        let two = Pid::from(2);
        let three = Pid::from(3);
        let mut state = State_ {
            day: 1,
            phase: Phase::Day {
                votes: HashMap::new(),
                blocks: HashMap::new(),
                pend_elect: None,
            },
            players: Players::from_registry(vec![
                (1, Role::TOWN),
                (2, Role::TOWN),
                (3, Role::MAFIA),
            ]),
            rules: Rules::default(),
            event_log: Vec::new(),
            update_thread: None,
        };

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

        state
            .vote(1, Some(Some(2)))
            .expect("Vote other should work");
        if let Phase::Day { votes, .. } = &state.phase {
            assert_eq!(votes.get(&one).unwrap(), &Some(two));
        } else {
            panic!("Expected Day phase");
        }

        state
            .vote(1, Some(Some(3)))
            .expect("Vote again should work");
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
        let mut state = State_ {
            day: 1,
            phase: Phase::Init,
            players: Players::from_registry(vec![
                (1, Role::TOWN),
                (2, Role::TOWN),
                (3, Role::MAFIA),
            ]),
            rules: Rules::default(),
            event_log: Vec::new(),
            update_thread: None,
        };

        let err = state
            .vote(1, Some(Some(2)))
            .expect_err("Vote in init should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
            pend_dawn: None,
        };

        let err = state
            .vote(1, Some(Some(3)))
            .expect_err("Vote in night should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::End { winner: Team::Town };

        let err = state
            .vote(1, Some(None))
            .expect_err("Vote in end should fail");
        assert!(matches!(err, Error::InvalidPhase { .. }));

        state.phase = Phase::Day {
            votes: HashMap::new(),
            blocks: HashMap::new(),
            pend_elect: None,
        };

        // Invalid voter
        let err = state
            .vote(4, Some(Some(2)))
            .expect_err("Invalid voter should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Invalid ballot

        let err = state
            .vote(1, Some(Some(4)))
            .expect_err("Invalid ballot should fail");

        assert!(matches!(err, Error::InvalidPlayer { .. }));

        // Ineffective vote

        state.vote(1, Some(Some(2))).expect("Vote should work");
        let err = state
            .vote(1, Some(Some(2)))
            .expect_err("Ineffective vote should fail");

        assert!(matches!(err, Error::IneffectiveVote));
    }
}
