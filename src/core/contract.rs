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
pub enum Task {
    Protect (PID),
    Assassinate (PID),
    ElectSelf(bool), // bool is success
}

pub struct Contract {
    holder: PID,
    task: Task,
}
// pub enum Contract {
//     Protect {
//         holder: PID,
//         charge: PID,
//         status: ChargeStatus,
//     },
//     Assassinate {
//         holder: PID,
//         charge: PID,
//         status: ChargeStatus,
//     },
//     Elect {
//         holder: PID,
//         status: IdiotStatus,
//     },
//     Survive {
//         holder: PID,
//         status: ChargeStatus,
//     },
// }

impl Contract {

    pub fn get_charge(&self) -> PID {
        match self.task {
            Task::Protect(pid) => pid.clone(),
            Task::Assassinate(pid) => pid.clone(),
            Task::ElectSelf (..) => self.holder.clone(),
        }
    }

    // pub fn description(&self) -> String {
    //     match self {
    //         Contract::Protect { charge, .. } => {
    //             format!(
    //                 "Your contract is to Protect. Your charge is {}. Keep them from dying!",
    //                 charge
    //             )
    //         }
    //         Contract::Assassinate { charge, .. } => {
    //             format!(
    //                 "Your contract is to Assassinate. Your charge is {}.  Cause their death!",
    //                 charge
    //             )
    //         }
    //         Contract::Elect { .. } => {
    //             format!("Your contract is.. to be Elected! 🙃 Win an election! ")
    //         }
    //         Contract::Survive { .. } => {
    //             format!("Your contract is to Survive. Stay alive!")
    //         }
    //     }
    // }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize /*Deserialize*/)]
pub enum ContractResult {
    Success { holder: PID },
    Failure { holder: PID },
}

impl Contract {
    pub fn charge_eliminated(&mut self, players: &mut Players, proxy: PID, comm: &Comm) {
        match self {
            Contract::Assassinate {
                holder,
                charge,
                status,
            } => {
                // check that holder is still alive
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

    pub fn charge_elected(&mut self, comm: &Comm) {
        match self {
            Contract::Elect { holder, status } => {
                // Update
                *status = IdiotStatus::Elected;
            }
            _ => {}
        }
    }

    pub fn check_win(&self) -> ContractResult {
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
