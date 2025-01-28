use crate::prelude::*;

use super::util::thresh;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Status {
    pub day: u32,
    pub phase: PhaseKind,
    pub votes: Option<HashMap<Choice, Vec<Pid>>>,
    pub count: HashMap<Team, usize>,
    pub players: Vec<Pid>,
    pub rules: Rules,
    pub names: HashMap<Pid, String>,
}

impl Status {
    pub fn brief(&self) -> String {
        let mut msg = String::new();
        msg.push_str(&format!("{} {}:\n  ", self.phase, self.day));
        msg.push_str("  Teams:\n");
        if let Some(town) = self.count.get(&Team::Town) {
            msg.push_str(&format!("    Town: {}\n", town));
        }
        if let Some(mafia) = self.count.get(&Team::Mafia) {
            msg.push_str(&format!("    Mafia: {}\n", mafia));
        }
        if let Some(rogue) = self.count.get(&Team::Rogue) {
            msg.push_str(&format!("    Rogue: {}\n", rogue));
        }
        msg
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let empty = "???".to_string();
        write!(f, "{} {}:\n", self.phase, self.day)?;

        // Display votes
        if let Some(vote_list) = &self.votes {
            let th = thresh(self.players.len(), &Some(Pid::from(0)));
            let pth = thresh(self.players.len(), &None);
            write!(f, "  Votes:\n")?;
            for pid in self.players.iter() {
                let votes = vote_list.get(&Some(*pid));
                if let Some(votes) = votes {
                    let name = self.names.get(pid).unwrap_or(&empty).to_string();
                    write!(f, "    {}({}/{}): ", name, votes.len(), th)?;
                    let names = votes
                        .into_iter()
                        .map(|pid| self.names.get(pid).unwrap_or(&empty).to_string())
                        .collect::<Vec<_>>();
                    let names_str = names.join(", ");
                    write!(f, "{names_str}\n")?;
                }
            }
            let votes = vote_list.get(&None);
            if let Some(votes) = votes {
                write!(f, "    Abstain({}/{}): ", votes.len(), pth)?;
                let names = votes
                    .into_iter()
                    .map(|pid| self.names.get(pid).unwrap_or(&empty).to_string())
                    .collect::<Vec<_>>();
                let names_str = names.join(", ");
                write!(f, "{names_str}\n")?;
            }
        }
        write!(f, "  Teams:\n")?;
        if let Some(town) = self.count.get(&Team::Town) {
            write!(f, "    Town: {}\n", town)?;
        }
        if let Some(mafia) = self.count.get(&Team::Mafia) {
            write!(f, "    Mafia: {}\n", mafia)?;
        }
        if let Some(rogue) = self.count.get(&Team::Rogue) {
            write!(f, "    Rogue: {}\n", rogue)?;
        }
        Ok(())
    }
}

impl State {
    pub fn status(&self, names: &HashMap<impl Into<Pid> + Copy, String>) -> Status {
        let names = names.into_iter().map(|(k, v)| ((*k).into(), v.clone())).collect();
        Status {
            day: self.day,
            phase: self.phase.kind(),
            votes: self.phase.vote_list().ok(),
            count: self.players.counts(Team::from),
            players: self.players.list(),
            rules: self.rules.clone(),
            names: names,
        }
    }
}
