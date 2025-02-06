use super::{phase::Votes, players::Players};
use crate::prelude::*;

pub const ELECTION_DELAY: Duration = Duration::from_secs(1);
pub const DAWN_DELAY: Duration = Duration::from_secs(1);

pub fn count_votes(votes: &Votes, players: &Players) -> HashMap<Choice, Vec<Pid>> {
    let mut vote_list: HashMap<Choice, Vec<Pid>> = HashMap::new();
    for (pid, _) in players.alive() {
        vote_list.insert(Some(pid), vec![]);
    }
    vote_list.insert(None, vec![]);
    for (voter, choice) in votes {
        vote_list.entry(*choice).or_default().push(*voter);
    }
    vote_list
}

pub fn thresh(n: usize, choice: &Choice) -> usize {
    if choice.is_some() {
        return (n / 2) + 1;
    } else {
        return (n + 1) / 2;
    }
}
