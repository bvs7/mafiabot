#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

#[macro_use]
extern crate enum_kinds;

mod prelude;

mod base;
pub use base::{Pid, Role, RoleKind, Team};

pub mod game;
pub mod rolegen;
pub mod rules;
pub mod state;
