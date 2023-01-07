mod game;
mod interface;
mod rules;
mod test;

use serde::Serialize;
use std::fmt::Debug;
use std::fs::File;

// TODO: decide exactly what to export!!
use game::*;
use interface::{action::*, error::*, event::*, *};

pub use game::Game;
pub use game::RawPID;
