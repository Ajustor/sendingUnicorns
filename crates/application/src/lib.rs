//! Application layer: the HTTP client's use cases.
//!
//! One use case per intent (R-ARCH-04). They talk to the outside world only
//! through the traits in [`ports`], which the infrastructure implements:
//! nothing here knows about a file format, an HTTP library or a window.

pub mod collections;
pub mod history;
pub mod ports;
pub mod runner;
pub mod sending;
pub mod settings;

#[cfg(test)]
mod fakes;
