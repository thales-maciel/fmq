//! Crate prelude

pub use crate::error::FmqError;

pub type Result<T> = core::result::Result<T, FmqError>;

pub use std::format as f;
