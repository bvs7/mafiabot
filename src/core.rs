mod game;
mod interface;
mod rules;
mod test;

use serde::Serialize;
use std::fmt::Debug;
use std::fs::File;

// TODO: decide exactly what to export!!
pub use game::*;
pub use interface::{action::*, error::*, event::*, *};

pub use game::{Game, Player, Players};
pub use rules::*;
