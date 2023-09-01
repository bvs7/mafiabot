//! Crate prelude

pub use kinded::Kinded;
pub use serde::Serialize;

pub use super::error::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub struct W<T>(pub T);

use std::format as f;
