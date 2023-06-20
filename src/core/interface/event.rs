use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<U: RawPID> {
    Init {
        game_id: usize,
    },
    Start {
        players: Vec<Player<U>>,
        contracts: Vec<Contract<U>>,
        phase: PhaseKind,
    },
    Day {
        day_no: usize,
        players: Vec<Player<U>>,
    },
    Vote {
        voter: Player<U>,
        ballot: Option<Player<U>>,
        former: Option<Option<Player<U>>>,
        threshold: usize,
        count: usize,
    },
    Retract {
        voter: Player<U>,
        former: Option<Option<Player<U>>>,
    },
    Reveal {
        celeb: Player<U>,
    },
    Election {
        electors: Vec<Player<U>>,
        ballot: Option<Player<U>>,
    },
    Night {
        night_no: usize,
        players: Vec<Player<U>>,
    },
    Target {
        actor: Player<U>,
        target: Option<Player<U>>,
    },
    Mark {
        killer: Player<U>,
        mark: Option<Player<U>>,
    },
    Dawn,
    Strip {
        stripper: Player<U>,
        blocked: Player<U>,
    },
    Block {
        blocked: Player<U>,
    },
    Save {
        doctor: Player<U>,
        saved: Player<U>,
    },
    Investigate {
        cop: Player<U>,
        suspect: Player<U>,
        role: Role,
    },
    Kill {
        killer: Player<U>,
        mark: Player<U>,
    },
    NoKill,
    Eliminate {
        player: Player<U>,
    },
    Refocus {
        new_contract: Contract<U>,
    },
    End {
        winner: Team,
        contract_results: Vec<ContractResult<U>>,
    },
}

impl<U: RawPID> Display for Event<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Init{game_id} => write!(f, "Init"),
            Event::Start {
                players,
                contracts,
                phase,
            } => write!(f, "Start: {:?} {:?} {:?}", players, contracts, phase),
            Event::Day { day_no, players } => write!(f, "Day {}: {:?}", day_no, players),
            Event::Vote {
                voter,
                ballot,
                former,
                threshold,
                count,
            } => write!(
                f,
                "Vote: {:?} {:?} {:?} {} {}",
                voter, ballot, former, threshold, count
            ),
            Event::Retract { voter, former } => write!(f, "Retract: {:?} {:?}", voter, former),
            Event::Reveal { celeb } => write!(f, "Reveal: {:?}", celeb),
            Event::Election { electors, ballot } => {
                write!(f, "Election: {:?} {:?}", electors, ballot)
            }
            Event::Night { night_no, players } => write!(f, "Night {}: {:?}", night_no, players),
            Event::Target { actor, target } => write!(f, "Target: {:?} {:?}", actor, target),
            Event::Mark { killer, mark } => write!(f, "Mark: {:?} {:?}", killer, mark),
            Event::Dawn => write!(f, "Dawn"),
            Event::Strip { stripper, blocked } => write!(f, "Strip: {:?} {:?}", stripper, blocked),
            Event::Block { blocked } => write!(f, "Block: {:?}", blocked),
            Event::Save { doctor, saved } => write!(f, "Save: {:?} {:?}", doctor, saved),
            Event::Investigate { cop, suspect, role } => {
                write!(f, "Investigate: {:?} {:?} {:?}", cop, suspect, role)
            }
            Event::Kill { killer, mark } => write!(f, "Kill: {:?} {:?}", killer, mark),
            Event::NoKill => write!(f, "NoKill"),
            Event::Eliminate { player } => write!(f, "Eliminate: {:?}", player),
            Event::Refocus { new_contract } => write!(f, "Refocus: {:?}", new_contract),
            Event::End {
                winner,
                contract_results,
            } => {
                write!(f, "End: {:?}, contracts: {:?}", winner, contract_results)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Init,
    Start,
    Day,
    Vote,
    Retract,
    Reveal,
    Election,
    Night,
    Target,
    Mark,
    Dawn,
    Strip,
    Block,
    Save,
    Investigate,
    Kill,
    NoKill,
    Eliminate,
    Refocus,
    End,
}

impl Event<u64> {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::Init{ .. } => EventKind::Init,
            Event::Start { .. } => EventKind::Start,
            Event::Day { .. } => EventKind::Day,
            Event::Vote { .. } => EventKind::Vote,
            Event::Retract { .. } => EventKind::Retract,
            Event::Reveal { .. } => EventKind::Reveal,
            Event::Election { .. } => EventKind::Election,
            Event::Night { .. } => EventKind::Night,
            Event::Target { .. } => EventKind::Target,
            Event::Mark { .. } => EventKind::Mark,
            Event::Dawn => EventKind::Dawn,
            Event::Strip { .. } => EventKind::Strip,
            Event::Block { .. } => EventKind::Block,
            Event::Save { .. } => EventKind::Save,
            Event::Investigate { .. } => EventKind::Investigate,
            Event::Kill { .. } => EventKind::Kill,
            Event::NoKill => EventKind::NoKill,
            Event::Eliminate { .. } => EventKind::Eliminate,
            Event::Refocus { .. } => EventKind::Refocus,
            Event::End { .. } => EventKind::End,
        }
    }
}
