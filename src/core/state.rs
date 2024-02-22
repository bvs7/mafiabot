
enum Choice<PID>
where
PID: Hash + Eq 
{
    Player(PID),
    Abstain,
}

enum Phase<PID>
where
PID: Hash + Eq 
{
    Day {
        votes: HashMap<PID, Choice>,
        blocks: HashMap<PID, List<PID>>,
    },
    Night {
        targets: HashMap<PID, Choice>,
        scheme: Option<(PID, Choice<PID>)>,
    },
    Eclipse {
        avenger: PID,
        hammer: PID,
        voters: HashSet<PID>,
    }
}



enum GameState<PID>
where
PID: Hash + Eq 
{
    Play {
        day_no: usize,
        players: HashMap<PID, Player>,
        phase: Phase,
    }
    End {
        winner: Team,
    }
}