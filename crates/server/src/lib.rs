//! The server's parts, as a library.
//!
//! `main.rs` is the program that serves the book; this library holds
//! everything it is made of. Splitting them lets a second program,
//! `primer-lint`, reuse the content loader, its checks and the Jev client
//! instead of copying them. `pub mod` makes a module visible outside the crate.

pub mod config;
pub mod content;
pub mod db;
pub mod jev;
pub mod judge;
pub mod names;
pub mod routes;
pub mod voices;
