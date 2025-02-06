use crate::prelude::*;

// We probably need two generics for start roles and known roles here?
// Maybe even a third for reveal on death...
// Alternatively, have one Rules trait of some sort with associated types!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Start {
        players: Vec<(Pid, Role)>,
        rules: Rules,
    },
    Day {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Night {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Eclipse {
        avenger: Pid,
        hammer: Pid,
        guilty: Vec<Pid>,
    },
    Vengeance {
        avenger: Pid,
        victim: Pid,
    },
    Vote {
        voter: Pid,
        ballot: Option<(Choice, usize)>,
        former: Option<(Choice, usize)>,
    },
    Reveal {
        celeb: Pid,
    },
    Election {
        choice: Choice,
        hammer: Pid,
        voters: Vec<Pid>,
    },
    Dawn, // Potentially note those who failed to do night actions
    Eliminate {
        player: Pid,
        role: RoleKind,
        context: Context,
    },
    Target {
        actor: Pid,
        choice: Choice,
    },
    Scheme {
        killer: Pid,
        mark: Choice,
    },
    Block {
        blocked: Pid,
        blockers: Vec<Pid>,
    },
    Save {
        saved: Pid,
        saviors: Vec<Pid>,
    },
    NoKill,
    Kill {
        killer: Pid,
        mark: Pid,
    },
    Investigate {
        cop: Pid,
        target: Pid,
        appears_mafia: bool,
    },
    Milk {
        milky: Pid,
        target: Pid,
    },
    End {
        winner: Team,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NightAction {}

use crate::state::night_action::NightAct;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event2 {
    Start { players: Vec<(Pid, Role)> },
    Day { day: u32, players: Vec<(Pid, Role)> },
    Night { day: u32, players: Vec<(Pid, Role)> },
    Eclipse { avenger: Pid, hammer: Pid, guilty: Vec<Pid> },
    Election { choice: Choice, hammer: Pid, vote_list: HashMap<Choice, Vec<Pid>> },
    Eliminate { player: Pid, role: RoleKind },
    Dawn { night_actions: Vec<NightAct> },
    Block { blocked: Pid, blockers: Vec<Pid> },
    Save { saved: Pid, saviors: Vec<Pid> },
    NoKill,
    Kill { killer: Pid, mark: Pid },
    Investigate { cop: Pid, target: Pid, appears_mafia: bool },
    Milk { milky: Pid, target: Pid },
    End { winner: Team },
    Reveal { player: Pid, role: RoleKind },
    Debug(usize),
}

impl std::fmt::Display for Event2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Event2::*;
        match self {
            Start { players } => write!(f, "Start: {:?}", players),
            Day { day, players } => write!(f, "Day {}: {:?}", day, players),
            Reveal { player, role } => write!(f, "Reveal: {} {}", player, role),
            Night { day, players } => write!(f, "Night {}: {:?}", day, players),
            Eclipse { avenger, hammer, guilty } => {
                write!(f, "Eclipse: {} {} {:?}", avenger, hammer, guilty)
            }
            Election { choice, hammer, vote_list } => {
                write!(
                    f,
                    "Election: {} {} {:?}",
                    choice.map(|pid| pid.to_string()).unwrap_or_else(|| "None".to_string()),
                    hammer,
                    vote_list
                )
            }
            Eliminate { player, role } => write!(f, "Eliminate: {} {}", player, role),
            Dawn { night_actions } => write!(f, "Dawn: {:?}", night_actions),
            Block { blocked, blockers } => write!(f, "Block: {} {:?}", blocked, blockers),
            Save { saved, saviors } => write!(f, "Save: {} {:?}", saved, saviors),
            NoKill => write!(f, "NoKill"),
            Kill { killer, mark } => write!(f, "Kill: {} {}", killer, mark),
            Investigate { cop, target, appears_mafia } => {
                write!(f, "Investigate: {} {} {}", cop, target, appears_mafia)
            }
            Milk { milky, target } => write!(f, "Milk: {} {}", milky, target),
            End { winner } => write!(f, "End: {:?}", winner),
            Debug(n) => write!(f, "Debug: {}", n),
        }
    }
}
