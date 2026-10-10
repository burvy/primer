//! The pure core of Primer: plain data and plain functions, no I/O.
//!
//! "Pure" means a function's result depends only on its arguments, and it
//! changes nothing outside itself. That makes it trivial to test, and lets
//! this crate build for both the server (native) and the browser (wasm).

mod generate;
mod learner;
mod look;
mod mistake;
mod page;
mod pattern;
pub mod policy;
mod problem;
mod problem_set;
mod skill;

pub use generate::generate;
pub use learner::{
    Attempt, Event, Learner, Outcome, Progress, RECENT, SLOW_MS, Strategy, WINS_NEEDED,
};
pub use look::Look;
pub use mistake::{MistakeKind, detect};
pub use page::Page;
pub use pattern::{Pattern, groups, walk};
pub use problem::{Blank, Op, Problem};
pub use problem_set::{MIN_LEN, ProblemSet, SET_LEN, generate_set};
pub use skill::{Regroup, SKILLS, Skill, Stage, Track};
