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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event2 {
    Start { players: Vec<(Pid, Role)>, rules: Rules },
    Day { day: u32, players: Vec<(Pid, Role)> },
    Night { day: u32, players: Vec<(Pid, Role)> },
    Eclipse { avenger: Pid, hammer: Pid, guilty: Vec<Pid> },
    Election { choice: Choice, hammer: Pid, vote_list: HashMap<Choice, Vec<Pid>> },
    Eliminate { player: Pid, role: Role },
    Dawn { night_actions: Vec<NightAction> },
    Block { blocked: Pid, blockers: Vec<Pid> },
    Save { saved: Pid, saviors: Vec<Pid> },
    NoKill,
    Kill { actor: Pid, target: Pid },
    Investigate { cop: Pid, target: Pid, appears_mafia: bool },
    Milk { milky: Pid, target: Pid },
    End { winner: Team },
    Reveal { celeb: Pid },
    Debug(usize),
}

pub enum ActionResp {
    Vote { voter: Pid, ballot: Option<(Choice, usize)>, former: Option<(Choice, usize)> },}
    Target { actor: Pid, choice: Choice },
    Scheme { killer: Pid, mark: Choice },
    Ok,
}
