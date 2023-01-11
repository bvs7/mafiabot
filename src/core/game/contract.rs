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
    Survive {
        holder: U,
        status: ChargeStatus,
    },
}

impl<U: RawPID> Contract<U> {
    pub fn new(holder: U, charge: U, offense: bool) -> Self {
        match offense {
            true if holder == charge => Contract::Elect {
                holder,
                status: IdiotStatus::Unelected,
            },
            false if holder == charge => Contract::Survive {
                holder,
                status: ChargeStatus::Alive,
            },
            true => Contract::Assassinate {
                holder,
                charge,
                status: ChargeStatus::Alive,
            },
            false => Contract::Protect {
                holder,
                charge,
                status: ChargeStatus::Alive,
            },
        }
    }
    pub fn get_holder(&self) -> U {
        match self {
            Contract::Protect { holder, .. } => *holder,
            Contract::Assassinate { holder, .. } => *holder,
            Contract::Elect { holder, .. } => *holder,
            Contract::Survive { holder, .. } => *holder,
        }
    }
    pub fn get_charge(&self) -> U {
        match self {
            Contract::Protect { charge, .. } => *charge,
            Contract::Assassinate { charge, .. } => *charge,
            Contract::Elect { holder, .. } => *holder,
            Contract::Survive { holder, .. } => *holder,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Contract::Protect { charge, .. } => {
                format!(
                    "Your contract is to Protect. Your charge is {}. Keep them from dying!",
                    charge
                )
            }
            Contract::Assassinate { charge, .. } => {
                format!(
                    "Your contract is to Assassinate. Your charge is {}.  Cause their death!",
                    charge
                )
            }
            Contract::Elect { .. } => {
                format!("Your contract is.. to be Elected! 🙃 Win an election! ")
            }
            Contract::Survive { .. } => {
                format!("Your contract is to Survive. Stay alive!")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum ContractResult<U: RawPID> {
    Success { holder: U },
    Failure { holder: U },
}

impl<U: RawPID> Contract<U> {
    pub fn charge_eliminated(&mut self, players: &mut Players<U>, proxy: U, comm: &Comm<U>) {
        match self {
            Contract::Assassinate {
                holder,
                charge,
                status,
            } => {
                if players.check(*holder).is_ok() {
                    // Refocus
                    if players.check(proxy).is_ok() {
                        *self = Contract::new(*holder, proxy, false)
                    } else {
                        *self = Contract::new(*holder, *holder, false)
                    }
                    comm.tx(Event::Refocus {
                        new_contract: self.clone(),
                    })
                } else {
                    // Update
                    *status = ChargeStatus::Dead;
                }
            }
            Contract::Protect {
                holder,
                charge,
                status,
            } => {
                if players.check(*holder).is_ok() {
                    // Refocus
                    if players.check(proxy).is_ok() {
                        *self = Contract::new(*holder, proxy, true)
                    } else {
                        *self = Contract::new(*holder, *holder, true)
                    }
                    comm.tx(Event::Refocus {
                        new_contract: self.clone(),
                    })
                } else {
                    // Update
                    *status = ChargeStatus::Dead;
                }
            }
            Contract::Survive { holder, status } => {
                *status = ChargeStatus::Dead;
            }
            _ => {}
        }
    }

    pub fn charge_elected(&mut self, comm: &Comm<U>) {
        match self {
            Contract::Elect { holder, status } => {
                // Update
                *status = IdiotStatus::Elected;
            }
            _ => {}
        }
    }

    pub fn check_win(&self) -> ContractResult<U> {
        match self {
            Contract::Assassinate { holder, status, .. } if *status == ChargeStatus::Dead => {
                ContractResult::Success { holder: *holder }
            }
            Contract::Protect { holder, status, .. } if *status == ChargeStatus::Alive => {
                ContractResult::Success { holder: *holder }
            }
            Contract::Elect { holder, status } if *status == IdiotStatus::Elected => {
                ContractResult::Success { holder: *holder }
            }
            Contract::Survive { holder, status } if *status == ChargeStatus::Alive => {
                ContractResult::Success { holder: *holder }
            }
            _ => ContractResult::Failure {
                holder: self.get_holder(),
            },
        }
    }
}
