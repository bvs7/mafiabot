use serde::Serialize;
use std::fmt::Debug;

use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum ChargeStatus {
    #[default]
    Alive,
    Dead,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum IdiotStatus {
    #[default]
    Unelected,
    Elected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Contract<U: RawPID> {
    Protect {
        holder: U,
        charge: U,
        status: ChargeStatus,
    },
    Assassinate {
        holder: U,
        charge: U,
        status: ChargeStatus,
    },
    Elect {
        holder: U,
        status: IdiotStatus,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum ContractResult<U: RawPID> {
    Win { holder: U },
    Loss { holder: U },
}

impl<U: RawPID> Contract<U> {
    pub fn refocus(&mut self, players: &Players<U>, died: U, proxy: U, comm: &Comm<U>) {
        match self {
            Contract::Assassinate {
                holder,
                charge,
                status,
            }
            | Contract::Protect {
                holder,
                charge,
                status,
            } if *charge == died => {
                // Check if alive
                if players.check(*holder).is_ok() {
                    let new_charge = if players.check(proxy).is_ok() {
                        proxy
                    } else {
                        *holder
                    };
                    *self = self.to_refocused(new_charge);
                    comm.tx(Event::Refocus {
                        new_contract: *self,
                    })
                } else {
                    *status = ChargeStatus::Dead;
                }
            }
            _ => {}
        }
    }

    pub fn to_refocused(&self, charge: U) -> Self {
        match self {
            Contract::Assassinate { holder, .. } => Contract::Protect {
                holder: *holder,
                charge,
                status: ChargeStatus::Alive,
            },
            Contract::Protect { holder, .. } => Contract::Assassinate {
                holder: *holder,
                charge,
                status: ChargeStatus::Alive,
            },
            _ => panic!("Should not refocus this contract"),
        }
    }

    pub fn check_win(&self) -> ContractResult<U> {
        match self {
            Contract::Assassinate { holder, status, .. } if *status == ChargeStatus::Dead => {
                ContractResult::Win { holder: *holder }
            }
            Contract::Protect { holder, status, .. } if *status == ChargeStatus::Alive => {
                ContractResult::Win { holder: *holder }
            }
            Contract::Elect { holder, status } if *status == IdiotStatus::Elected => {
                ContractResult::Win { holder: *holder }
            }
            Contract::Assassinate { holder, .. }
            | Contract::Protect { holder, .. }
            | Contract::Elect { holder, .. } => ContractResult::Loss { holder: *holder },
        }
    }
}
