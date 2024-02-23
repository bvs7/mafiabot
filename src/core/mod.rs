// don't warn about unused imports
#![allow(dead_code, unused_variables, unused_imports)]

pub mod base;
pub mod error;
pub mod events;
pub mod roles;
pub mod timer;

use base::{Choice, ID};
use error::CoreError;
use events::{send, Action, Event, EventOutput};
use roles::{Role, RoleKind, Team};
use timer::Timer;

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender};
use std::sync::{Arc, Mutex};
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

#[derive(Debug, Clone, Copy)]
pub struct Rules {}

// Maintains historical data about the game
// Used for revealing information about the game
#[derive(Debug, Clone)]
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
    Init,
    Day {
        votes: HashMap<PID, Choice<PID>>, // voter -> choice
        blocks: HashMap<PID, Vec<PID>>,   // blocked -> blockers
        elect_timer: Option<Timer<Choice<PID>>>,
    },
    Night {
        targets: HashMap<PID, Choice<PID>>, // actor -> target
        scheme: Option<(PID, Choice<PID>)>, // actor -> (target, choice)
        dawn_timer: Option<Timer<()>>,
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

impl<PID: ID> State<PID> {
    pub fn new(players: HashMap<PID, Role>) -> Self {
        let day_no = 0;
        let phase = Phase::Init;
        State {
            day_no,
            players,
            phase,
        }
    }
}

fn check_election_specific<PID: ID>(
    votes: &HashMap<PID, Choice<PID>>,
    n: usize,
    candidate: Choice<PID>,
) -> Option<Vec<PID>> {
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
        return Some(voters);
    }

    return None;
}

#[derive(Debug)]
pub struct Core<PID: ID, GID: ID> {
    id: GID,
    state: State<PID>,
    rules: Rules,
    event_tx: EventOutput<PID>,
    action_rx: Receiver<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
    action_tx: Sender<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
}

/*
Core game loop:
- Take in an action + response channel
- process the action and return the response

Additionaly, the action could spawn some kind of timer. That timer might itself throw an action
*/

impl<'a, PID: ID, GID: ID> Core<PID, GID> {
    pub fn new(
        id: GID,
        players: HashMap<PID, Role>,
        rules: Rules,
        event_tx: EventOutput<PID>,
        action_rx: Receiver<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
        action_tx: Sender<(Action<PID>, Sender<Result<(), CoreError<PID>>>)>,
    ) -> Self {
        let day_no = 0;

        let state = State::new(players);

        let mut core = Core {
            id,
            state,
            rules,
            event_tx,
            action_rx,
            action_tx,
        };

        core.start().expect("No send error during init");
        return core;
    }

    /// Consumes self
    pub fn start_thread(mut self) {
        thread::spawn(move || {
            self.run();
        });
    }

    pub fn run(&mut self) {
        loop {
            let (action, response) = match self.action_rx.recv() {
                Ok(t) => t,
                Err(RecvError) => {
                    panic!("Action channel error!")
                }
            };

            // Do action
            let resp = match action {
                Action::Vote { voter, choice } => self.vote(voter, Some(choice)),
                Action::Unvote { voter } => self.vote(voter, None),
                Action::Reveal { player } => self.reveal(player),
                Action::Target { actor, target } => self.target(actor, target),
                Action::Scheme { actor, mark } => self.scheme(actor, mark),
                Action::Elect { candidate } => self.elect(candidate),
                Action::Dawn => self.dawn(),
                Action::Close => break,
            };

            // Send response
            response.send(Ok(())).unwrap();
        }
        self.event_tx.send(Event::Close).unwrap();
    }

    pub fn start(&mut self) -> Result<(), CoreError<PID>> {
        // For now assume start even
        let n = self.state.players.len();
        if n % 2 == 0 {
            self.to_night()?;
        } else {
            self.to_day(None)?;
        }
        Ok(())
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
        let n = self.state.players.len();
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

        if former_ballot == ballot {
            return Ok(());
        }

        send(
            &self.event_tx,
            Event::Vote {
                voter,
                ballot,
                former_ballot,
            },
        )?;

        // Check for a former election and whether it still has quorum
        if let Some(choice) = former_ballot {
            if let None = check_election_specific(votes, n, choice) {
                // elect_timer.cancel();
            }
        }

        // Check for a new election
        if let Some(candidate) = ballot {
            if let Some(_) = check_election_specific(votes, n, candidate) {
                // Set election timer
                let action_tx = self.action_tx.clone();
                *elect_timer = Some(Timer::new(
                    SystemTime::now() + Duration::from_secs(1),
                    candidate,
                    Duration::from_millis(100),
                    Box::new(move |candidate| {
                        let (tx, rx) = std::sync::mpsc::channel();
                        action_tx.send((Action::Elect { candidate }, tx)).unwrap();
                        rx.recv().unwrap().expect("Election failed!");
                    }),
                ));
            }
        }

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
        let Phase::Day { .. } = self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        send(&self.event_tx, Event::Reveal { player, role })?;
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
        send(&self.event_tx, Event::Target { actor, target })?;
        // TODO: schedule dawn check for the future
        self.check_dawn()?;
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
        send(&self.event_tx, Event::Scheme { actor, mark })?;
        self.check_dawn()?;
        Ok(())
    }

    fn check_end(&self) -> Option<Team> {
        let n = self.state.players.len();
        let n_mafia = self
            .state
            .players
            .values()
            .filter(|&&role| role.team() == Team::Mafia)
            .count();

        if n_mafia == 0 {
            // Town wins!
            return Some(Team::Town);
        } else if n - n_mafia <= n_mafia {
            // Mafia wins!
            return Some(Team::Mafia);
        }
        return None;
    }

    fn eliminate(&mut self, player: PID) -> Result<bool, CoreError<PID>> {
        let Some(&role) = self.state.players.get(&player) else {
            return Err(CoreError::InvalidPlayer { player });
        };
        send(&self.event_tx, Event::Eliminate { player, role })?;
        self.state.players.remove(&player);
        // Check for end of game
        if let Some(winner) = self.check_end() {
            self.state.phase = Phase::End { winner };
            send(&self.event_tx, Event::End { winner })?;
            return Ok(true);
        }
        Ok(false)
    }

    fn elect(&mut self, candidate: Choice<PID>) -> Result<(), CoreError<PID>> {
        // Ensure the phase is Day
        let n = self.state.players.len();
        let Phase::Day { votes, .. } = &mut self.state.phase else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Day;
            return Err(CoreError::InvalidPhase { actual, expected });
        };
        if let Choice::Player(player) = candidate {
            if !self.state.players.contains_key(&player) {
                return Err(CoreError::InvalidPlayer { player });
            }
            // Triple check for election?
            let Some(voters) = check_election_specific(votes, n, candidate) else {
                return Err(CoreError::ExpectedElection { candidate });
            };
            send(
                &self.event_tx,
                Event::Election {
                    choice: candidate,
                    voters,
                },
            )?;
            // TODO: check for IDIOT!
            if self.eliminate(player)? {
                // Game Over!
                return Ok(());
            }
        }
        self.to_night()?;
        Ok(())
    }

    fn dawn(&mut self) -> Result<(), CoreError<PID>> {
        let Phase::Night {
            targets, scheme, ..
        } = &self.state.phase
        else {
            let actual = self.state.phase.kind();
            let expected = PhaseKind::Night;
            return Err(CoreError::InvalidPhase { actual, expected });
        };

        send(&self.event_tx, Event::Dawn)?;

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
            role.night_action(actor, target, &mut dawn_state, &self.event_tx)?;
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
                            &self.event_tx,
                            Event::EvidentBlock {
                                blocked: savior,
                                blockers: blockers.clone(),
                            },
                        )?;
                        continue;
                    }
                    saved = true;
                    send(&self.event_tx, Event::EvidentSave { savior, mark })?;
                }
            }
            if !saved {
                dawn_state.killed.insert(mark);
                send(&self.event_tx, Event::Kill { killer, mark })?;
            }
        }
        if dawn_state.killed.len() > 0 {
            let mut game_over = false;
            for &player in &dawn_state.killed {
                game_over = self.eliminate(player)? | game_over;
            }
            if game_over {
                return Ok(());
            }
        } else {
            send(&self.event_tx, Event::NoNightKill)?;
        }

        // Perform post-scheme actions
        while night_actions.peek().is_some() {
            let NightAction {
                actor,
                role,
                target,
                ..
            } = night_actions.pop().unwrap();
            role.night_action(actor, target, &mut dawn_state, &self.event_tx)?;
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
            &self.event_tx,
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
            dawn_timer: None,
        };
        send(
            &self.event_tx,
            Event::Night {
                day_no: self.state.day_no,
            },
        )?;
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

    fn check_dawn(&mut self) -> Result<bool, CoreError<PID>> {
        let Phase::Night {
            targets,
            scheme,
            dawn_timer,
        } = &mut self.state.phase
        else {
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
        // Schedule dawn!
        let action_tx = self.action_tx.clone();
        *dawn_timer = Some(Timer::new(
            SystemTime::now() + Duration::from_secs(1),
            (),
            Duration::from_millis(100),
            Box::new(move |_| {
                let (tx, rx) = std::sync::mpsc::channel();
                action_tx.send((Action::Dawn, tx)).unwrap();
                rx.recv().unwrap().expect("Dawn failed!");
            }),
        ));

        return Ok(true);
    }

    // fn send(&self, event: Event<PID>) -> Result<(), CoreError<PID>> {
    //     self.events
    //         .send(event)
    //         .map_err(|err| CoreError::EventSendError(err))
    // }
}
