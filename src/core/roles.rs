use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use crate::core::base::ID;
use crate::interface::{CoreError, Event, EventOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind)]
#[enum_kind(RoleKind)]
pub enum Role<PID: ID> {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MAFIA,
    STRIPPER,
    IDIOT(bool), // Bool is if IDIOT was elected
    SURVIVOR,
    GUARD(PID),
    AGENT(PID),
}

impl<PID: ID> Role<PID> {
    pub fn is_targeting(&self) -> bool {
        match self {
            Role::COP => true,
            Role::DOCTOR => true,
            Role::STRIPPER => true,
            _ => false,
        }
    }

    pub fn is_scheming(&self) -> bool {
        return self.team() == Team::Mafia;
    }

    pub fn contract(&self) -> Option<PID> {
        return match self {
            Role::GUARD(charge) => Some(*charge),
            Role::AGENT(charge) => Some(*charge),
            _ => None,
        };
    }

    pub fn validate_scheming(&self) -> Result<(), CoreError<PID>> {
        if !self.is_scheming() {
            return Err(CoreError::ExpectedSchemingRole {
                role: RoleKind::from(self),
            });
        }
        Ok(())
    }
    // Larger priority happens first at dawn. Equal priorities can happen in any order.
    // Positive happens before mafia scheme is resolved. Negative happens after mafia scheme is resolved.
    pub fn night_action_priority(&self) -> Option<i8> {
        return match self {
            Role::COP => Some(-1),
            Role::DOCTOR => Some(1),
            Role::STRIPPER => Some(2),
            _ => None,
        };
    }

    pub async fn night_action(
        &self,
        actor: PID,
        target: PID,
        dawn_state: &DawnState<PID>,
        events: &EventOutput<PID>,
    ) -> Result<Vec<DawnStateChange<PID>>, CoreError<PID>> {
        match self {
            Role::COP => {
                // if cop was killed, do nothing
                if dawn_state.killed.contains_key(&actor) {
                    return Ok(vec![]);
                }
                // if target was killed, do nothing
                if dawn_state.killed.contains_key(&target) {
                    return Ok(vec![]);
                }
                if let Some((&blocked, blockers)) = dawn_state.blocks.get_key_value(&actor) {
                    let blockers = blockers.clone();
                    events
                        .send(Event::EvidentBlock { blocked, blockers })
                        .await?;
                    return Ok(vec![]);
                }
                events.send(Event::Investigate { actor, target }).await?;
            }
            Role::DOCTOR => {
                events.send(Event::Save { actor, target }).await?;
                return Ok(vec![DawnStateChange::Save { actor, target }]);
            }
            Role::STRIPPER => {
                events.send(Event::Block { actor, target }).await?;
                return Ok(vec![DawnStateChange::Block { actor, target }]);
            }

            _ => {}
        }
        Ok(vec![])
    }

    pub fn team(&self) -> Team {
        return match self {
            Role::TOWN | Role::COP | Role::DOCTOR | Role::CELEB => Team::Town,
            Role::MAFIA | Role::STRIPPER => Team::Mafia,
            Role::IDIOT(_) | Role::SURVIVOR | Role::GUARD(_) | Role::AGENT(_) => Team::Rogue,
        };
    }
    pub fn kind(&self) -> RoleKind {
        return RoleKind::from(self);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}

// mutated by Roles at dawn
#[derive(Debug)]
pub struct DawnState<PID: ID> {
    pub blocks: HashMap<PID, Vec<PID>>,
    pub saves: HashMap<PID, Vec<PID>>,
    pub killed: HashMap<PID, PID>, // marks -> killer
}

pub enum DawnStateChange<PID: ID> {
    Block { actor: PID, target: PID },
    Save { actor: PID, target: PID },
    Kill { killer: PID, mark: PID },
}

impl<PID: ID> DawnState<PID> {
    pub fn apply_changes(&mut self, changes: Vec<DawnStateChange<PID>>) {
        for change in changes {
            match change {
                DawnStateChange::Block { actor, target } => {
                    self.blocks.entry(target).or_insert(Vec::new()).push(actor);
                }
                DawnStateChange::Save { actor, target } => {
                    self.saves.entry(target).or_insert(Vec::new()).push(actor);
                }
                DawnStateChange::Kill { killer, mark } => {
                    self.killed.insert(mark, killer);
                }
            }
        }
    }
}
