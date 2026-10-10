//! The Judge: the part of the book that uses judgment, not arithmetic.
//!
//! `detect` in the core spots mistakes that have a clear signature. For the
//! rest, and for reading what a child typed, the server asks a judge. It only
//! knows the `Judge` *trait*: a promise that "something with these methods
//! exists". The real one is `JevJudge` (in `jev.rs`); tests and runs without a
//! key use [`FakeJudge`], which needs no network.

use std::sync::Arc;

use async_trait::async_trait;
use primer_core::{MistakeKind, Problem, Strategy};

/// What a judge concluded about one answer. Every field has a safe "don't
/// know" value, and [`Judgment::UNKNOWN`] is all of them at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Judgment {
    pub mistake: MistakeKind,
    pub strategy: Strategy,
    pub frustrated: bool,
}

impl Judgment {
    pub const UNKNOWN: Judgment = Judgment {
        mistake: MistakeKind::Unknown,
        strategy: Strategy::Unknown,
        frustrated: false,
    };
}

/// `async fn` inside a trait cannot yet be used through `dyn Judge` (a trait
/// object: "any Judge, decided at run time"), so the `async_trait` crate
/// rewrites it into a plain function that returns a boxed future. `Send +
/// Sync` mean it may be shared between the server's worker threads.
#[async_trait]
pub trait Judge: Send + Sync {
    /// Judges one answer. `given` is `None` for "I don't know";
    /// `explanation` is what the child typed for "How did you know?", if
    /// anything. The explanation is only read here, never kept.
    async fn judge(
        &self,
        problem: &Problem,
        given: Option<u32>,
        explanation: Option<&str>,
    ) -> Judgment;

    /// Could this typed hero name identify a real child (a real person's
    /// name, a school, an address)? `true` means "refuse it".
    async fn identifying(&self, hero_name: &str) -> bool;

    /// Names the slip in `given`, or `Unknown` if it cannot.
    ///
    /// A *default method*: every judge gets it for free, built on `judge`.
    async fn mistake(&self, problem: &Problem, given: u32) -> MistakeKind {
        self.judge(problem, Some(given), None).await.mistake
    }
}

/// Always says "don't know", and lets every name through (the code rule on
/// names still applies). Stands in for the real judge.
pub struct FakeJudge;

#[async_trait]
impl Judge for FakeJudge {
    async fn judge(
        &self,
        _problem: &Problem,
        _given: Option<u32>,
        _explanation: Option<&str>,
    ) -> Judgment {
        Judgment::UNKNOWN
    }

    async fn identifying(&self, _hero_name: &str) -> bool {
        false
    }
}

/// A shareable handle to whichever judge is in use.
pub type SharedJudge = Arc<dyn Judge>;
