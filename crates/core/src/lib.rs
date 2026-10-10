//! The pure core of Primer: plain data and plain functions, no I/O.
//!
//! "Pure" means a function's result depends only on its arguments, and it
//! changes nothing outside itself. That makes it trivial to test, and lets
//! this crate build for both the server (native) and the browser (wasm).

mod look;
mod problem;
mod skill;

pub use look::Look;
pub use problem::{Op, Problem};
pub use skill::{SKILLS, Skill, Stage, Track};
