//! What the child sees next: the policy's answer.

use crate::learner::Strategy;
use crate::mistake::MistakeKind;
use crate::problem::Problem;
use crate::skill::Stage;

/// One screen of the book. Each variant carries exactly the data its screen
/// needs, so a `Practice` page can never be missing its problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    /// Ada's lesson. `again` is true when it is offered after struggling.
    Lesson {
        skill: &'static str,
        lesson: String,
        again: bool,
    },
    /// One problem from a set.
    Practice {
        skill: &'static str,
        set_id: String,
        problem: Problem,
        stage: Stage,
        /// Show the hint for this mistake (after stepping back).
        hint: Option<MistakeKind>,
        /// Shown easier than usual, with kind words, after a struggle.
        easier: bool,
        /// A better way to try, when the child's explanation showed a slow one.
        nudge: Option<Strategy>,
    },
    /// Every chapter the child can reach is mastered: the story page.
    Story,
}
