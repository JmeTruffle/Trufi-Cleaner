//! Trufi Cleaner core.
//!
//! Everything in this crate is OS-agnostic. Anything that needs OS APIs
//! (registry, recycle bin, startup entries, elevation) goes through the
//! [`platform::Platform`] trait, implemented in the `trufi-platform` crate.
//!
//! Cleaning knowledge lives in data, not code: rule packs are TOML files
//! (see `rules/` at the repository root) parsed by [`rules`].

pub mod advisor;
pub mod cleaner;
pub mod error;
pub mod health;
pub mod large_files;
pub mod leftovers;
pub mod model;
pub mod monitor;
pub mod platform;
pub mod rules;
pub mod safety;
pub mod scanner;
pub mod util;

pub use error::{Error, Result};
