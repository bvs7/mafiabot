// don't warn about unused imports
#![allow(dead_code, unused_variables, unused_imports)]

pub mod base;
pub mod error;
pub mod events;
pub mod roles;
pub mod timer;

use base::{Choice, ID};
use error::CoreError;
use events::{send, Event, EventOutput};
use roles::{Role, RoleKind, Team};
use timer::Timer;

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::mpsc::{SendError, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NightAction<PID: ID> {
    actor: PID,
    role: Role,
    target: PID,
    priority: i8,
}

impl<PID: ID> PartialOrd for NightAction<PID> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

impl<PID: ID> Ord for NightAction<PID> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

#[derive(Debug)]
pub struct Rules {}

// Maintains historical data about the game
// Used for revealing information about the game
#[derive(Debug)]
pub struct Stats<PID: ID> {
    role_history: HashMap<PID, Vec<Role>>,
}

impl<PID: ID> Stats<PID> {
    fn new() -> Self {
        Stats {
            role_history: HashMap::new(),
        }
    }
}

// mutated by Roles at dawn
#[derive(Debug)]
pub struct DawnState<PID: ID> {
    blocks: HashMap<PID, Vec<PID>>,
    saves: HashMap<PID, Vec<PID>>,
    killed: HashSet<PID>,
}

/**
* Alarms are used to schedule events in the future
Things we want:
- Alarm is scheduled for a future time.
- Alarm is scheduled with a callback function.
- The new thread will sleep until the scheduled time then call the callback function.
- The callback function can be cancelled.
- the new thread will check if the alarm has been cancelled before calling the callback function.
- The Alarm can be serialized and deserialized. (i.e. restarted on boot)

The alarm itself is just a thread with a mutex the cancelled flag and data

*/

#[derive(EnumKind, Debug)]
#[enum_kind(PhaseKind)]
pub enum Phase<PID: ID> {
    Day {
        votes: HashMap<PID, Choice<PID>>, // voter -> choice
        blocks: HashMap<PID, Vec<PID>>,   // blocked -> blockers
        elect_timer: Option<Timer<Choice<PID>>>,
    },
    Night {
        targets: HashMap<PID, Choice<PID>>, // actor -> target
        scheme: Option<(PID, Choice<PID>)>, // actor -> (target, choice)
    },
    Eclipse {
        avenger: PID,
        hammer: PID,
        options: HashSet<PID>,
    },
    End {
        winner: Team,
    },
}

impl<PID: ID> Phase<PID> {
    fn new(players: &HashMap<PID, Role>, rules: &Rules) -> Self {
        Phase::Day {
            votes: HashMap::new(),
            blocks: HashMap::new(),
            elect_timer: None,
        }
    }

    fn kind(&self) -> PhaseKind {
        return PhaseKind::from(self);
        // return match self {
        //     Phase::Day { .. } => PhaseKind::Day,
        //     Phase::Night { .. } => PhaseKind::Night,
        //     Phase::Eclipse { .. } => PhaseKind::Eclipse,
        //     Phase::End { .. } => PhaseKind::End,
        // };
    }
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// enum PhaseKind {
//     Day,
//     Night,
//     Eclipse,
//     End,
// }

#[derive(Debug)]
struct State<PID: ID> {
    day_no: u32,
    players: HashMap<PID, Role>,
    phase: Phase<PID>,
}

fn check_election_specific<PID: ID>(
    votes: &HashMap<PID, Choice<PID>>,
    n: usize,
    candidate: Choice<PID>,
) -> Option<(Choice<PID>, Vec<PID>)> {
    let threshold = match candidate {
        Choice::Player(_) => n / 2 + 1,
        Choice::Abstain => (n + 1) / 2,
    };

    // count the votes for each player
    let mut voters: Vec<PID> = Vec::new();
    for (&voter, &choice) in votes {
        if choice == candidate {
            voters.push(voter);
        }
    }
    if voters.len() >= threshold {
        return Some((candidate, voters));
    }

    return None;
}

#[derive(Debug)]
pub struct Core<PID: ID, GID: ID> {
    id: GID,
    state: State<PID>,
    rules: Rules,
    stats: Stats<PID>,
    event_out: EventOutput<PID>,
}

impl<PID: ID, GID: ID> Core<PID, GID> {
    // New
    //  from scratch
    //    inputs: id, players, rules, events

    pub fn new(
        id: GID,
        players: HashMap<PID, Role>,
        rules: Rules,
        event_out: EventOutput<PID>,
    ) -> Self {
        let day_no = 0;
        let phase = Phase::new(&players, &rules);
        let state = State {
            day_no,
            players,
            phase,
        };
        let stats = Stats::new();
        Core {
            id,
            state,
            rules,
            stats,
            event_out,
        }
    }

    fn validate_player(&self, player: PID) -> Result<Role, CoreError<PID>> {
        let Some(&role) = self.state.players.get(&player) else {
            return Err(CoreError::InvalidPlayer { player });
        };
        Ok(role)
    }

    pub fn vote(&mut self, voter: PID, ballot: Option<Choice<PID>>) -> Result<(), CoreError<PID>> {
        let _ = self.validate_player(voter)?;
        if let Some(Choice::Player(player)) = ballot {
            let _ = self.validate_player(player)?;
        }
        // Check if the phase is day
        let Phase::Day {
            votes, elect_timer, ..
        } = &mut self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        // Update votes
        let former_ballot = match ballot {
            Some(choice) => votes.insert(voter, choice),
            None => votes.remove(&voter),
        };
        send(
            &self.event_out,
            Event::Vote {
                voter,
                ballot,
                former_ballot,
            },
        )?;

        // Check for a former election and whether it still has quorum
        todo!();

        // Check for a new election
        todo!();

        Ok(())
    }

    pub fn reveal(&self, player: PID) -> Result<(), CoreError<PID>> {
        let role = self.validate_player(player)?;
        // Check if the role is a celeb
        if role != Role::CELEB {
            let actual = role.kind();
            return Err(CoreError::ExpectedCeleb { actual });
        }
        // Check that Phase is Day
        let Phase::Day { .. } = &self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        send(&self.event_out, Event::Reveal { player, role })?;
        Ok(())
    }

    pub fn target(&mut self, actor: PID, target: Choice<PID>) -> Result<(), CoreError<PID>> {
        let role = self.validate_player(actor)?;
        role.validate_targeting()?;
        if let Choice::Player(player) = target {
            let _ = self.validate_player(player)?;
        }

        // Check if the phase is night
        let Phase::Night { targets, .. } = &mut self.state.phase else {
            return Err(CoreError::InvalidPhase {
                actual: self.state.phase.kind(),
                expected: PhaseKind::Night,
            });
        };
        let former_target = targets.insert(actor, target);
        send(&self.event_out, Event::Target { actor, target })?;
        // TODO: schedule dawn check for the future
        if self.check_dawn()? {
            self.dawn()?;
        }
        Ok(())
    }

    pub fn scheme(&mut self, actor: PID, mark: Choice<PID>) -> Result<(), CoreError<PID>> {
        let role = self.validate_player(actor)?;
        role.validate_scheming()?;
        if let Choice::Player(player) = mark {
            let _ = self.validate_player(player)?;
        }

        // Check if the phase is night
        let Phase::Night { scheme, .. } = &mut self.state.phase else {
            return Err(CoreError::InvalidPhase {
                actual: self.state.phase.kind(),
                expected: PhaseKind::Night,
            });
        };
        scheme.replace((actor, mark));
        send(&self.event_out, Event::Scheme { actor, mark })?;
        // TODO: schedule dawn check for the future
        if self.check_dawn()? {
            self.dawn()?;
        }
        Ok(())
    }

    fn eliminate(&mut self, player: PID) -> Result<(), CoreError<PID>> {
        let Some(&role) = self.state.players.get(&player) else {
            return Err(CoreError::InvalidPlayer { player });
        };
        send(&self.event_out, Event::Eliminate { player, role })?;
        self.state.players.remove(&player);
        Ok(())
    }

    fn elect(&mut self, candidate: Choice<PID>, voters: Vec<PID>) -> Result<(), CoreError<PID>> {
        // Ensure the phase is Day
        let Phase::Day { .. } = &mut self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };
        if let Choice::Player(player) = candidate {
            if !self.state.players.contains_key(&player) {
                return Err(CoreError::InvalidPlayer { player });
            }
            send(
                &self.event_out,
                Event::Election {
                    choice: candidate,
                    voters,
                },
            )?;
            // TODO: check for IDIOT!
            self.eliminate(player)?;
        }
        self.to_night()?;
        Ok(())
    }

    fn dawn(&mut self) -> Result<(), CoreError<PID>> {
        let Phase::Night { targets, scheme } = &self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Night;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        send(&self.event_out, Event::Dawn)?;

        // Sort targets by priority
        let mut night_actions: BinaryHeap<NightAction<PID>> = BinaryHeap::new();
        for (&actor, &target) in targets {
            let Some(&role) = self.state.players.get(&actor) else {
                return Err(CoreError::InvalidPlayer { player: actor });
            };
            let Choice::Player(target) = target else {
                continue;
            };
            let Some(priority) = role.night_action_priority() else {
                panic!("Role {:?} has no priority!", role);
            };
            let action = NightAction {
                actor,
                role,
                target,
                priority,
            };
            night_actions.push(action);
        }

        let mut dawn_state: DawnState<PID> = DawnState {
            blocks: HashMap::new(),
            saves: HashMap::new(),
            killed: HashSet::new(),
        };

        // Perform pre-scheme actions
        while night_actions.peek().is_some_and(|f| f.priority > 0) {
            let NightAction {
                actor,
                role,
                target,
                ..
            } = night_actions.pop().unwrap();
            role.night_action(actor, target, &mut dawn_state, &self.event_out)?;
        }

        // Perform scheme
        if let &Some((killer, Choice::Player(mark))) = scheme {
            // TODO: if killer was blocked or killed, do nothing
            let mut saved = false;
            // Check for saviors
            if let Some(saviors) = dawn_state.saves.get(&mark) {
                for &savior in saviors {
                    // Check if doctor was killed or blocked?
                    if dawn_state.killed.contains(&savior) {
                        continue;
                    }
                    if let Some(blockers) = dawn_state.blocks.get(&savior) {
                        send(
                            &self.event_out,
                            Event::EvidentBlock {
                                blocked: savior,
                                blockers: blockers.clone(),
                            },
                        )?;
                        continue;
                    }
                    saved = true;
                    send(&self.event_out, Event::EvidentSave { savior, mark })?;
                }
            }
            if !saved {
                dawn_state.killed.insert(mark);
                send(&self.event_out, Event::Kill { killer, mark })?;
            }
        }

        if dawn_state.killed.len() > 0 {
            for &player in &dawn_state.killed {
                self.eliminate(player)?;
            }
        } else {
            send(&self.event_out, Event::NoNightKill)?;
        }

        // Perform post-scheme actions
        while night_actions.peek().is_some() {
            let NightAction {
                actor,
                role,
                target,
                ..
            } = night_actions.pop().unwrap();
            role.night_action(actor, target, &mut dawn_state, &self.event_out)?;
        }

        self.to_day(Some(dawn_state.blocks))?;
        Ok(())
    }

    fn to_day(&mut self, blocks: Option<HashMap<PID, Vec<PID>>>) -> Result<(), CoreError<PID>> {
        self.state.day_no += 1;
        let blocks = blocks.unwrap_or(HashMap::new());
        self.state.phase = Phase::Day {
            votes: HashMap::new(),
            blocks,
            elect_timer: None,
        };
        send(
            &self.event_out,
            Event::Day {
                day_no: self.state.day_no,
            },
        )?;
        Ok(())
    }

    fn to_night(&mut self) -> Result<(), CoreError<PID>> {
        self.state.phase = Phase::Night {
            targets: HashMap::new(),
            scheme: None,
        };
        Ok(())
    }

    // fn check_election_general(&self) -> Option<(Choice<PID>, Vec<PID>)> {
    //     if let Phase::Day { votes, .. } = &self.state.phase {
    //         // Check for an election
    //         // determine kill and no-kill thresholds
    //         let kill_threshold = self.state.players.len() / 2 + 1;
    //         let nokill_threshold = (self.state.players.len() + 1) / 2;
    //         // count the votes for each player
    //         let mut choices: HashMap<Choice<PID>, Vec<PID>> = HashMap::new();
    //         for (&voter, &choice) in votes {
    //             choices.entry(choice).or_insert(Vec::new()).push(voter);
    //         }

    //         // check for an election (if a player choice has reached the kill threshold or if the Abstain choice has reached the no-kill threshold)
    //         let election = choices.into_iter().find(|(choice, voters)| match choice {
    //             Choice::Player(_) => voters.len() >= kill_threshold,
    //             Choice::Abstain => voters.len() >= nokill_threshold,
    //         });

    //         if let Some((choice, voters)) = election {
    //             return Some((choice, voters));
    //         }
    //     }
    //     return None;
    // }

    fn check_dawn(&self) -> Result<bool, CoreError<PID>> {
        let Phase::Night { targets, scheme } = &self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Night;
            return Err(CoreError::InvalidPhase { actual, expected });
        };
        if let None = scheme {
            return Ok(false);
        }
        // Check that every targeting role has a target
        for (player, role) in &self.state.players {
            if role.is_targeting() {
                if !targets.contains_key(player) {
                    return Ok(false);
                }
            }
        }
        return Ok(true);
    }

    // fn send(&self, event: Event<PID>) -> Result<(), CoreError<PID>> {
    //     self.events
    //         .send(event)
    //         .map_err(|err| CoreError::EventSendError(err))
    // }
}

// Implement the ID trait for u32
impl ID for u32 {}
