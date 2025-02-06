use crate::prelude::*;

use super::util::*;
use super::EventTx;

#[derive(Debug, Clone)]
pub enum Command<P> {
    Vote { voter: P, ballot: Option<Option<P>> },
    Reveal { actor: P },
    Target { actor: P, choice: Option<P> },
    Scheme { killer: P, mark: Option<P> },
    Status,
}

#[derive(Debug, Clone)]
pub enum Validated {
    Action(Action),
    Resp(ActionResp),
}

impl From<Action> for Validated {
    fn from(action: Action) -> Self {
        Validated::Action(action)
    }
}

impl From<ActionResp> for Validated {
    fn from(resp: ActionResp) -> Self {
        Validated::Resp(resp)
    }
}

#[derive(Debug, Clone)]
pub enum ActionResp {
    Vote { voter: Pid, ballot: Option<(Choice, usize)>, former: Option<(Choice, usize)> },
    Target { actor: Pid, choice: Choice },
    Scheme { killer: Pid, mark: Choice },
    Vengeance { avenger: Pid, victim: Pid },
    Status(State),
    Ok,
}

impl State {
    pub fn validate_command<P: Into<Pid> + Copy + std::fmt::Debug>(
        &self,
        cmd: Command<P>,
    ) -> Result<Validated, Error> {
        use Command::*;
        let result = match cmd {
            Vote { voter, ballot } => self.validate_vote(voter, ballot)?.into(),
            Target { actor, choice } => self.validate_target(actor, choice)?.into(),
            Scheme { killer, mark } => self.validate_scheme(killer, mark)?.into(),
            Reveal { actor } => self.validate_reveal(actor)?.into(),
            Status => ActionResp::Status(self.clone()).into(),
        };
        Ok(result)
    }

    fn validate_vote(
        &self,
        voter: impl Into<Pid> + Copy,
        ballot: Option<Option<impl Into<Pid> + Copy>>,
    ) -> Result<Action, Error> {
        let voter = self.players.validate(voter)?;
        let ballot = self.players.validate_ballot(ballot)?;
        let Phase::Day { votes, .. } = &self.phase else {
            if matches!(self.phase, Phase::Eclipse { .. }) {
                return self.validate_eclipse_vote(voter, ballot);
            }
            return self.phase.expected(PhaseKind::Day);
        };
        let former = votes.iter().find(|(v, _)| v == &voter).map(|(_, c)| c.clone());
        if former == ballot {
            return Err(Error::IneffectiveAction);
        }
        Ok(Action::Vote(voter, ballot))
    }

    fn validate_reveal(&self, celeb: impl Into<Pid> + Copy) -> Result<Action, Error> {
        let celeb = self.players.validate(celeb)?;
        let role = self.players.get_role(celeb);
        if role != Role::CELEB {
            return Err(Error::ExpectedCeleb { actual: role.kind() });
        }
        if !matches!(self.phase, Phase::Day { .. }) {
            return self.phase.expected(PhaseKind::Day);
        }
        Ok(Action::Reveal(celeb))
    }
    // TODO: check stripper
    fn validate_target(
        &self,
        actor: impl Into<Pid> + Copy,
        choice: Option<impl Into<Pid> + Copy>,
    ) -> Result<Action, Error> {
        let actor = self.players.validate(actor)?;
        let choice = self.players.validate_choice(choice)?;
        let role = self.players.get_role(actor);
        if !role.is_targeting() {
            return Err(Error::ExpectedTargetingRole { actual: role.kind() });
        }
        let Phase::Night { targets, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Night);
        };
        if targets.get(&actor).is_some() {
            return Err(Error::IneffectiveAction);
        }
        Ok(Action::Target(actor, choice))
    }

    fn validate_scheme(
        &self,
        killer: impl Into<Pid> + Copy,
        mark: Option<impl Into<Pid> + Copy>,
    ) -> Result<Action, Error> {
        let killer = self.players.validate(killer)?;
        let mark = self.players.validate_choice(mark)?;
        let role = self.players.get_role(killer);
        if !role.is_scheming() && mark.is_some() {
            return Err(Error::ExpectedSchemingRole { actual: role.kind() });
        }
        let Phase::Night { scheme, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Night);
        };
        if scheme.is_some() {
            return Err(Error::IneffectiveAction);
        }
        Ok(Action::Scheme(killer, mark))
    }

    fn validate_eclipse_vote(&self, voter: Pid, ballot: Ballot) -> Result<Action, Error> {
        let Phase::Eclipse { avenger, guilty, .. } = &self.phase else {
            return self.phase.expected(PhaseKind::Eclipse);
        };
        if voter != *avenger {
            return Err(Error::IneffectiveAction);
        }
        match ballot {
            Some(Some(victim)) => {
                if victim == *avenger {
                    return Err(Error::IneffectiveAction);
                }
                if !guilty.contains(&victim) {
                    return Err(Error::IneffectiveAction);
                }
                return Ok(Action::EclipseVote(victim));
            }
            Some(None) => return Err(Error::IneffectiveAction),
            None => return Err(Error::IneffectiveAction),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    Vote(Pid, Ballot),
    Reveal(Pid),
    Target(Pid, Choice),
    Scheme(Pid, Choice),
    EclipseVote(Pid),
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Action::*;
        match self {
            Vote(voter, ballot) => {
                let b = match ballot {
                    Some(Some(pid)) => pid.to_string(),
                    Some(None) => "Nokill".to_string(),
                    None => "None".to_string(),
                };
                write!(f, "Vote: {} -> {}", voter, b)
            }
            Reveal(actor) => write!(f, "Reveal: {}", actor),
            Target(actor, choice) => {
                let c = match choice {
                    Some(pid) => pid.to_string(),
                    None => "None".to_string(),
                };
                write!(f, "Target: {} -> {}", actor, c)
            }
            Scheme(killer, mark) => {
                let c = match mark {
                    Some(pid) => pid.to_string(),
                    None => "None".to_string(),
                };
                write!(f, "Scheme: {} -> {}", killer, c)
            }
            EclipseVote(victim) => write!(f, "EclipseVote: {}", victim),
        }
    }
}

impl State {
    pub fn perform_action(&mut self, action: Action, tx: &EventTx) -> ActionResp {
        use Action::*;
        match action {
            Vote(voter, ballot) => self.vote(voter, ballot, tx),
            Target(actor, choice) => self.target(actor, choice, tx),
            Scheme(killer, mark) => self.scheme(killer, mark, tx),
            Reveal(celeb) => self.reveal(celeb, tx),
            EclipseVote(victim) => self.eclipse_vote(victim, tx),
        }
    }

    fn vote(&mut self, voter: Pid, ballot: Ballot, tx: &EventTx) -> ActionResp {
        let Phase::Day { votes, .. } = &mut self.phase else {
            panic!("Expected Day phase");
        };
        let former_idx = votes.iter().position(|(v, _)| v == &voter);
        let former = former_idx.map(|i| votes.remove(i).1);
        if let Some(choice) = ballot {
            votes.push((voter, choice));
        }
        let vote_list = count_votes(&votes, &self.players);

        let ballot_check = ballot.clone().map(|c| (c, vote_list.get(&c).unwrap().len()));
        let former_check = former.clone().map(|c| (c, vote_list.get(&c).unwrap().len()));

        self.check_election(voter, ballot, vote_list, tx);
        ActionResp::Vote { voter, ballot: ballot_check, former: former_check }
    }

    fn reveal(&mut self, celeb: Pid, tx: &EventTx) -> ActionResp {
        let Phase::Day { blocks, .. } = &self.phase else {
            panic!("Expected Day phase");
        };
        if let Some(blockers) = blocks.get(&celeb) {
            let _ = tx.send(Event2::Block { blocked: celeb, blockers: blockers.clone() });
        } else {
            let _ = tx.send(Event2::Reveal { player: celeb, role: Role::CELEB.kind() });
        }
        ActionResp::Ok
    }

    fn check_election(
        &mut self,
        voter: Pid,
        ballot: Ballot,
        vote_list: HashMap<Choice, Vec<Pid>>,
        tx: &EventTx,
    ) {
        let Phase::Day { elect, .. } = &mut self.phase else {
            panic!("Expected Day phase");
        };
        let n = self.players.n();
        if let Some((choice, hammer, time)) = elect {
            if vote_list.get(&choice).unwrap().len() < thresh(n, &choice) {
                // Election averted
                *elect = None;
            }
        }
        if let Some(choice) = ballot {
            let voters = vote_list.get(&choice).unwrap();
            println!("Checking election, {:?}, {:?}, {:?}", n, thresh(n, &choice), voters.len());
            if voters.len() >= thresh(n, &choice) {
                let hammer = voter;
                let time = Local::now() + ELECTION_DELAY;
                *elect = Some((choice, hammer, time));
            }
        }
    }

    fn target(&mut self, actor: Pid, choice: Choice, tx: &EventTx) -> ActionResp {
        let Phase::Night { targets, .. } = &mut self.phase else {
            panic!("Expected Night phase");
        };
        targets.insert(actor, choice);
        self.check_dawn(tx);
        ActionResp::Target { actor, choice }
    }

    fn scheme(&mut self, killer: Pid, mark: Choice, tx: &EventTx) -> ActionResp {
        let Phase::Night { scheme, .. } = &mut self.phase else {
            panic!("Expected Night phase");
        };
        *scheme = Some((killer, mark));
        self.check_dawn(tx);
        ActionResp::Scheme { killer, mark }
    }

    fn check_dawn(&mut self, tx: &EventTx) {
        let Phase::Night { targets, scheme, dawn } = &mut self.phase else {
            panic!("Expected Night phase");
        };
        if dawn.is_some() {
            return;
        }
        let mut ready = true;
        if scheme.is_none() {
            ready = false;
        }
        for (pid, role) in self.players.alive() {
            if role.is_targeting() && targets.get(&pid).is_none() {
                ready = false;
            }
        }
        if ready {
            let time = Local::now() + DAWN_DELAY;
            *dawn = Some(time);
        }
    }

    fn eclipse_vote(&mut self, victim: Pid, tx: &EventTx) -> ActionResp {
        let Phase::Eclipse { avenger, hammer, vengeance, .. } = &mut self.phase else {
            panic!("Expected Eclipse phase");
        };
        let avenger = *avenger;
        let hammer = *hammer;
        *vengeance = Some(victim);
        ActionResp::Vengeance { avenger, victim }
    }
}
