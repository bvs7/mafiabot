use serde::Serialize;
use std::fmt::{Debug, Display};



// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
// pub enum Choice {
//     Player(Pidx),
//     Abstain,
// }

// impl Choice {
//     pub fn is_player(&self) -> Option<Pidx> {
//         if let Choice::Player(p) = self {
//             Some(*p)
//         } else {
//             None
//         }
//     }
//     pub fn as_opt(&self) -> Option<Pidx> {
//         if let Choice::Player(p) = self {
//             Some(*p)
//         } else {
//             None
//         }
//     }
// }
// impl Into<Option<Pidx>> for Choice {
//     fn into(self) -> Option<Pidx> {
//         if let Choice::Player(p) = self {
//             Some(p)
//         } else {
//             None
//         }
//     }
// }
// impl Choice {
//     pub fn to_p(&self, players: &Vec<Player>) -> Option<Player> {
//         match self {
//             Choice::Player(p) => Some(players[*p].clone()),
//             Choice::Abstain => None,
//         }
//     }
// }

// pub type Action<U> = (U, Choice<U>);
// pub type Election = (Vec<Pidx>, Choice<Pidx>);
