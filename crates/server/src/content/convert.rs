//! Turning checked file data into what the rest of the server uses: lessons
//! become [`BeatDto`]s for the web app, written sets become core
//! [`ProblemSet`]s for the policy.
//!
//! Each function returns `Err` with a list of messages when something is
//! wrong, so the writer sees every problem in an entry at once.

use primer_api::{BeatDto, NoteDto, SetEndDto};
use primer_core::{ProblemSet, Skill};

use super::check::{pattern_for, problem_for};
use super::feedback::{Hint, Praise, Story, fill_in};
use super::math::to_html;
use super::schema::{
    Beat, Hint as HintFile, Lesson, Praise as PraiseFile, ProblemSet as SetFile, Show,
    WordProblem,
};
use super::voice::{EXCEPTIONAL, GENTLE, Line, beat_voice, note_voice, set_name, speakable};

/// All the beats of a lesson, in order, across all its steps.
///
/// `flat_map` walks every step, then every beat in it, as one long run.
/// Errors are collected from every beat before giving up.
pub fn lesson_beats(skill: &Skill, lesson: &Lesson) -> Result<Vec<BeatDto>, Vec<String>> {
    let mut beats = Vec::new();
    let mut errors = Vec::new();
    if lesson.step.is_empty() {
        errors.push("a lesson needs at least one step".to_string());
    }
    for (step_number, step) in lesson.step.iter().enumerate() {
        // The step's example is checked once, then shared by its beats.
        let example = match step.example.as_deref().map(|e| problem_for(skill, e)) {
            Some(Err(error)) => {
                errors.push(format!("example: {error}"));
                None
            }
            Some(Ok(problem)) => Some(problem),
            None => None,
        };
        if step.beat.is_empty() {
            errors.push("a step needs at least one beat".to_string());
        }
        for (beat_number, beat) in step.beat.iter().enumerate() {
            // Numbers count from 1 in recording names.
            let place = (lesson.id.as_str(), step_number + 1, beat_number + 1);
            match beat_dto(example, beat, place) {
                Ok(dto) => beats.push(dto),
                Err(error) => errors.push(error),
            }
        }
    }
    if errors.is_empty() {
        Ok(beats)
    } else {
        Err(errors)
    }
}

/// One beat as the web app wants it. `(lesson, step, number)` is where it
/// sits, which names its recording.
///
/// Four small translations happen on the way:
/// - The words to read aloud are worked out from the raw text (`voice`).
/// - `text` and `formal` become HTML, with `$...$` turned into MathML.
/// - An equation beat with no `label` writes its step's `example` out in
///   full, like `3 + 4 = 7`.
/// - For a `swap` beat the file lists the pouches in their order *after* the
///   swap (the picture's order *before* it), so the two are exchanged here.
fn beat_dto(
    example: Option<primer_core::Problem>,
    beat: &Beat,
    (lesson, step, number): (&str, usize, usize),
) -> Result<BeatDto, String> {
    let html = |text: &str| to_html(text);
    let label = beat.label.clone().or_else(|| match (beat.show, example) {
        (Show::Equation, Some(problem)) => Some(written_out(&problem)),
        _ => None,
    });
    let (mut left, mut right) = (beat.left, beat.right);
    if beat.swap == Some(true) {
        std::mem::swap(&mut left, &mut right);
    }
    Ok(BeatDto {
        text: html(&beat.text)?,
        formal: beat.formal.as_deref().map(html).transpose()?,
        show: beat.show.word().to_string(),
        count: beat.count,
        adding: beat.adding,
        at: beat.at,
        hop: beat.hop,
        input: beat.input,
        output: beat.output,
        highlight: beat.highlight.map(|h| h.word().to_string()),
        label,
        open: beat.open,
        left,
        right,
        join: beat.join,
        swap: beat.swap,
        total: beat.total,
        image: beat.image.clone(),
        voice: Some(beat_voice(
            lesson,
            step,
            number,
            &beat.text,
            beat.formal.as_deref(),
        )),
    })
}

/// A line of the book as the page wants it: both wordings as HTML (maths as
/// MathML), and how Ada reads them. `id` names the recordings. Every
/// unreadable wording is reported, not only the first.
pub(super) fn note(id: &str, text: &str, formal: Option<&str>) -> Result<NoteDto, Vec<String>> {
    let text_html = to_html(text);
    let formal_html = formal.map(to_html).transpose();
    match (text_html, formal_html) {
        (Ok(text_html), Ok(formal_html)) => Ok(NoteDto {
            text: text_html,
            formal: formal_html,
            voice: note_voice(id, text, formal),
        }),
        (text_html, formal_html) => {
            let errors = [text_html.err(), formal_html.err()];
            Err(errors.into_iter().flatten().collect())
        }
    }
}

/// A word problem as a story for its sum, with `{a}` and `{b}` filled in.
pub fn story(problem: &WordProblem) -> Result<Story, Vec<String>> {
    let (a, b) = (problem.a, problem.b);
    let text = fill_in(&problem.text, a, b);
    let formal = problem.formal.as_ref().map(|f| fill_in(f, a, b));
    Ok(Story {
        id: problem.id.clone(),
        note: note(&problem.id, &text, formal.as_deref())?,
    })
}

/// A hint as the server keeps it.
pub fn hint(hint: &HintFile) -> Result<Hint, Vec<String>> {
    Ok(Hint {
        look: hint.look,
        stage: hint.stage,
        mistake: hint.mistake,
        note: note(&hint.id, &hint.text, hint.formal.as_deref())?,
    })
}

/// A line of praise as the server keeps it.
pub fn praise(praise: &PraiseFile) -> Result<Praise, Vec<String>> {
    Ok(Praise {
        look: praise.look,
        when: praise.when,
        note: note(&praise.id, &praise.text, praise.formal.as_deref())?,
    })
}

/// What Ada says when a set ends: its question, then its notice. `None` if
/// the set has neither. (Their wording was already checked by
/// [`problem_set`], so a line that cannot be read is simply left out.)
pub fn set_end(set: &SetFile) -> Option<SetEndDto> {
    let question = set
        .question
        .as_deref()
        .and_then(|text| note(&format!("{}.question", set.id), text, None).ok());
    let notice = set.notice.as_deref().and_then(|text| {
        note(&format!("{}.notice", set.id), text, set.formal_notice.as_deref()).ok()
    });
    (question.is_some() || notice.is_some()).then_some(SetEndDto { question, notice })
}

/// `3 + 4` as the full equation `3 + 4 = 7`. A problem that already has its
/// box inside prints in full (`3 + □ = 7`), so it is used as is.
fn written_out(problem: &primer_core::Problem) -> String {
    match problem.blank() {
        primer_core::Blank::Answer => format!("{problem} = {}", problem.whole()),
        _ => problem.to_string(),
    }
}

/// The lines Ada reads aloud for a set: its question and its notice (and the
/// formal notice for the Exceptional look).
pub fn set_lines(set: &SetFile) -> Vec<Line> {
    let line = |part: &str, look: &'static str, text: &String| Line {
        name: set_name(&set.id, part, look),
        look,
        speech: speakable(text),
    };
    [
        set.question.as_ref().map(|t| line("question", GENTLE, t)),
        set.notice.as_ref().map(|t| line("notice", GENTLE, t)),
        set.formal_notice
            .as_ref()
            .map(|t| line("notice", EXCEPTIONAL, t)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// A written set as the core wants it. Every problem must parse and keep the
/// skill's promise, and the pattern (if any) must be one of the skill's.
pub fn problem_set(skill: &'static Skill, set: &SetFile) -> Result<ProblemSet, Vec<String>> {
    let mut errors = Vec::new();
    if set.problems.is_empty() {
        errors.push("a set needs at least one problem".to_string());
    }
    let mut problems = Vec::new();
    for text in &set.problems {
        match problem_for(skill, text) {
            Ok(problem) => problems.push(problem),
            Err(error) => errors.push(error),
        }
    }
    let pattern = match set.pattern.as_deref().map(|id| pattern_for(skill, id)) {
        Some(Err(error)) => {
            errors.push(error);
            None
        }
        Some(Ok(pattern)) => Some(pattern),
        None => None,
    };
    // The sentences must be readable too, even though nothing shows them yet.
    for text in [&set.question, &set.notice, &set.formal_notice]
        .into_iter()
        .flatten()
    {
        if let Err(error) = to_html(text) {
            errors.push(error);
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(ProblemSet {
        id: set.id.clone(),
        skill: skill.id,
        problems,
        pattern,
        notice: set.notice.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::check::parse_problem;

    #[test]
    fn an_example_is_written_out_in_full() {
        let label = |text: &str| written_out(&parse_problem(text).unwrap());
        assert_eq!(label("3 + 4"), "3 + 4 = 7");
        assert_eq!(label("3 + □ = 10"), "3 + □ = 10");
    }
}
