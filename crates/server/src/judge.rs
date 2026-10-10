//! The Judge: who names a wrong answer that code could not recognise.
//!
//! `detect` in the core spots mistakes that have a clear signature. For the
//! rest, a later slice asks an AI (Jev). The server only knows the `Judge`
//! *trait*: a promise that "something with a `mistake` method exists". Tests
//! and local runs use [`FakeJudge`], which needs no network or key.

use std::sync::Arc;

use async_trait::async_trait;
use primer_core::{MistakeKind, Problem};

/// `async fn` inside a trait cannot yet be used through `dyn Judge` (a trait
/// object: "any Judge, decided at run time"), so the `async_trait` crate
/// rewrites it into a plain function that returns a boxed future. `Send +
/// Sync` mean it may be shared between the server's worker threads.
#[async_trait]
pub trait Judge: Send + Sync {
    /// Names the slip in `given`, or `Unknown` if it cannot.
    async fn mistake(&self, problem: &Problem, given: u32) -> MistakeKind;
}

/// Always says `Unknown`. Stands in for the real judge.
pub struct FakeJudge;

#[async_trait]
impl Judge for FakeJudge {
    async fn mistake(&self, _problem: &Problem, _given: u32) -> MistakeKind {
        MistakeKind::Unknown
    }
}

/// A shareable handle to whichever judge is in use.
pub type SharedJudge = Arc<dyn Judge>;
