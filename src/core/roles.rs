use std::fmt::Debug;
use std::hash::Hash;

use crate::base::ID;
use crate::core::{Core, CoreError, DawnState};
use crate::events::{send, Event, EventOutput};

// Create a trait PID that implements Eq, Hash, and Copy

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind)]
#[enum_kind(RoleKind)]
pub enum Role {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MAFIA,
    STRIPPER,
}

impl Role {
    pub fn is_targeting(&self) -> bool {
        match self {
            Role::TOWN => false,
            Role::COP => true,
            Role::DOCTOR => true,
            Role::CELEB => false,
            Role::MAFIA => false,
            Role::STRIPPER => true,
        }
    }

    pub fn validate_targeting<PID: ID>(&self) -> Result<(), CoreError<PID>> {
        if !self.is_targeting() {
            return Err(CoreError::ExpectedTargetingRole {
                role: RoleKind::from(self),
            });
        }
        Ok(())
    }

    pub fn is_scheming(&self) -> bool {
        return self.team() == Team::Mafia;
    }

    pub fn validate_scheming<PID: ID>(&self) -> Result<(), CoreError<PID>> {
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
            Role::TOWN => None,
            Role::CELEB => None,
            Role::MAFIA => None,
            Role::COP => Some(-1),
            Role::DOCTOR => Some(1),
            Role::STRIPPER => Some(2),
        };
    }

    pub fn night_action<PID: ID>(
        &self,
        actor: PID,
        target: PID,
        dawn_state: &mut DawnState<PID>,
        events: &EventOutput<PID>,
    ) -> Result<(), CoreError<PID>> {
        match self {
            Role::COP => {
                // if cop was killed, do nothing
                if dawn_state.killed.contains(&actor) {
                    return Ok(());
                }
                // if target was killed, do nothing
                if dawn_state.killed.contains(&target) {
                    return Ok(());
                }
                if let Some((&blocked, blockers)) = dawn_state.blocks.get_key_value(&actor) {
                    let blockers = blockers.clone();
                    send(events, Event::EvidentBlock { blocked, blockers })?;
                    return Ok(());
                }
                send(events, Event::Investigate { actor, target })?;
            }
            Role::DOCTOR => {
                dawn_state
                    .saves
                    .entry(target)
                    .or_insert(Vec::new())
                    .push(actor);

                send(events, Event::Save { actor, target })?;
            }
            Role::STRIPPER => {
                dawn_state
                    .blocks
                    .entry(target)
                    .or_insert(Vec::new())
                    .push(actor);

                send(events, Event::Block { actor, target })?;
            }

            _ => {}
        }
        Ok(())
    }

    pub fn team(&self) -> Team {
        return match self {
            Role::TOWN | Role::COP | Role::DOCTOR | Role::CELEB => Team::Town,
            Role::MAFIA | Role::STRIPPER => Team::Mafia,
        };
    }
    pub fn kind(&self) -> RoleKind {
        return RoleKind::from(self);
        // return match self {
        //     Role::TOWN => RoleKind::TOWN,
        //     Role::COP => RoleKind::COP,
        //     Role::CELEB => RoleKind::CELEB,
        //     Role::DOCTOR => RoleKind::DOCTOR,
        //     Role::MAFIA => RoleKind::MAFIA,
        //     Role::STRIPPER => RoleKind::STRIPPER,
        // };
    }
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// enum RoleKind {
//     TOWN,
//     COP,
//     DOCTOR,
//     CELEB,
//     MAFIA,
//     STRIPPER,
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}
