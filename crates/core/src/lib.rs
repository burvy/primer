//! The pure core of Primer: plain data and plain functions, no I/O.
//!
//! "Pure" means a function's result depends only on its arguments, and it
//! changes nothing outside itself. That makes it trivial to test, and lets
//! this crate build for both the server (native) and the browser (wasm).

mod generate;
mod look;
mod mistake;
mod problem;
mod skill;

pub use generate::generate;
pub use look::Look;
pub use mistake::{MistakeKind, detect};
pub use problem::{Blank, Op, Problem};
pub use skill::{Regroup, SKILLS, Skill, Stage, Track};
