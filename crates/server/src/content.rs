//! The written book: loads `content/*.toml`, checks it, and hands the rest of
//! the server what it needs.
//!
//! - `schema` says what a file may contain (serde types).
//! - `check` holds the pure checks (problems parse, sums fit the skill, ids
//!   are unique, what is not written yet).
//! - `convert` turns checked data into lesson beats and core problem sets.
//! - `math` turns `$...$` LaTeX into MathML.
//!
//! This file is the thin edge that ties them together. [`Content::load`]
//! reads the folder; [`Content::from_sources`] does everything else from
//! plain text, so tests need no files.

pub mod check;
mod convert;
pub mod feedback;
mod math;
pub mod schema;
pub mod voice;

use std::collections::HashMap;
use std::path::Path;

use primer_api::{BeatDto, NoteDto, SetEndDto};
use primer_core::policy::{Chapter, Step};
use primer_core::{Blank, Look, MistakeKind, Op, Problem, SKILLS, Skill, Stage};
use rand::Rng;

pub use check::ContentError;
use feedback::{Facts, Hint, Praise, Story, mistake_tag, stage_tag};
use schema::{ChapterFile, LookTag, WordsFile};

/// What Ada says about answers in one chapter.
#[derive(Default)]
struct Sayings {
    hints: Vec<Hint>,
    praise: Vec<Praise>,
    stories: Vec<Story>,
}

/// Everything loaded from `content/`, already checked.
pub struct Content {
    /// The chapters each look sees. A lesson or set marked `look = "gentle"`
    /// is left out of the Exceptional list, and the other way round.
    gentle: Vec<Chapter>,
    exceptional: Vec<Chapter>,
    /// Beats of every lesson, found by (skill id, lesson id).
    lessons: HashMap<(String, String), Vec<BeatDto>>,
    /// For each skill: its first lesson's id and the chapter's title.
    openers: HashMap<String, (String, String)>,
    /// Hints, praise and word problems of each skill's chapter.
    sayings: HashMap<String, Sayings>,
    /// What Ada says when a written set ends, found by the set's id.
    set_ends: HashMap<String, SetEndDto>,
    /// What is not written yet. Not errors: the server prints them at start.
    pub gaps: Vec<String>,
    /// Every line Ada reads aloud, in book order: the list for the voice actor.
    pub lines: Vec<voice::Line>,
}

impl Content {
    /// The hint for a wrong answer to a problem shown at `stage`: the one
    /// written for that mistake, else any hint for the stage, else none.
    /// "I don't know" has no mistake, so it is read as an `Unknown` one.
    pub fn hint(
        &self,
        skill: &str,
        look: Look,
        stage: Stage,
        mistake: MistakeKind,
    ) -> Option<NoteDto> {
        let hints = &self.sayings.get(skill)?.hints;
        let found = feedback::pick_hint(hints, look, stage_tag(stage), mistake_tag(mistake))?;
        Some(found.note.clone())
    }

    /// A word of praise for a right answer, chosen by `facts` with `rng`.
    pub fn praise(
        &self,
        skill: &str,
        look: Look,
        facts: Facts,
        rng: &mut impl Rng,
    ) -> Option<NoteDto> {
        let lines = &self.sayings.get(skill)?.praise;
        let moments = feedback::moments(facts);
        let found = feedback::pick_praise(lines, look, &moments, rng)?;
        Some(found.note.clone())
    }

    /// The story to tell for `problem` at `stage`, if the book has a word
    /// problem for this very sum at this stage. Only a sum with its answer
    /// missing has a story.
    pub fn story(&self, skill: &str, look: Look, stage: Stage, problem: &Problem) -> Option<NoteDto> {
        if problem.op() != Op::Add || problem.blank() != Blank::Answer {
            return None;
        }
        let stories = &self.sayings.get(skill)?.stories;
        let sum = (problem.a(), problem.b());
        let found = feedback::pick_story(stories, look, stage_tag(stage), sum)?;
        Some(found.note.clone())
    }

    /// What Ada says when the written set `set_id` ends, if she says anything.
    pub fn set_end(&self, set_id: &str) -> Option<SetEndDto> {
        self.set_ends.get(set_id).cloned()
    }

    /// The chapters to hand the policy for a child in this `look`.
    pub fn chapters(&self, look: Look) -> &[Chapter] {
        match look {
            Look::Gentle => &self.gentle,
            Look::Exceptional => &self.exceptional,
        }
    }

    /// The beats of one lesson, or none if there is no such lesson.
    pub fn beats(&self, skill: &str, lesson: &str) -> &[BeatDto] {
        self.lessons
            .get(&(skill.to_string(), lesson.to_string()))
            .map_or(&[], Vec::as_slice)
    }

    /// The opener page for `lesson`, if it is the first lesson of its
    /// chapter: the chapter's number (from 1, in ladder order) and title.
    pub fn opener(&self, skill: &str, lesson: &str) -> Option<(u32, &str)> {
        let (first, title) = self.openers.get(skill)?;
        let number = SKILLS.iter().position(|s| s.id == skill)? + 1;
        (first == lesson).then_some((number as u32, title.as_str()))
    }

    /// Reads every `.toml` file in `dir` and checks it all.
    ///
    /// Files are read in name order, so the result never depends on the
    /// order the operating system lists them in.
    pub fn load(dir: &Path) -> Result<Content, Vec<ContentError>> {
        let folder = dir.display().to_string();
        let fail = |error: std::io::Error| vec![ContentError::new(&folder, "", error.to_string())];
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(fail)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        paths.sort();
        let mut sources = Vec::new();
        for path in paths {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let text = std::fs::read_to_string(&path).map_err(fail)?;
            sources.push((name.to_string(), text));
        }
        Content::from_sources(&sources)
    }

    /// Checks and builds the content from `(file name, file text)` pairs.
    ///
    /// Which file is which is decided by its name: `words.toml` is the
    /// vocabulary, a name starting with a digit (`02-add.toml`) is a chapter,
    /// and anything else (`_template.toml`, `hero_names.toml`) is left alone.
    ///
    /// Errors are collected, not thrown on the first: the writer gets the
    /// whole list in one run.
    pub fn from_sources(sources: &[(String, String)]) -> Result<Content, Vec<ContentError>> {
        let mut errors = Vec::new();
        let mut chapters: Vec<(String, ChapterFile)> = Vec::new();
        let mut words: Option<WordsFile> = None;

        for (name, text) in sources {
            if name == "words.toml" {
                match toml::from_str::<WordsFile>(text) {
                    Ok(file) => {
                        errors.extend(check::check_words(name, &file));
                        words = Some(file);
                    }
                    Err(error) => errors.push(ContentError::new(name, "", error.to_string())),
                }
            } else if name.starts_with(|c: char| c.is_ascii_digit()) {
                match toml::from_str::<ChapterFile>(text) {
                    Ok(file) => chapters.push((name.clone(), file)),
                    Err(error) => errors.push(ContentError::new(name, "", error.to_string())),
                }
            }
        }

        errors.extend(check::duplicate_ids(&all_ids(&chapters)));
        let mut built = Built::default();
        for (name, chapter) in &chapters {
            built.add_chapter(name, chapter, &mut errors);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let lines = built.lines();
        Ok(Content {
            gentle: built.chapters(Look::Gentle),
            exceptional: built.chapters(Look::Exceptional),
            lessons: built.lessons,
            openers: built.openers,
            sayings: built.sayings,
            set_ends: built.set_ends,
            gaps: check::coverage(&chapters, words.as_ref()),
            lines,
        })
    }
}

/// Every id in every chapter file, with its file, for the uniqueness check.
fn all_ids(chapters: &[(String, ChapterFile)]) -> Vec<(String, String)> {
    let mut ids = Vec::new();
    for (name, c) in chapters {
        let mut add = |id: &String| ids.push((name.clone(), id.clone()));
        c.lesson.iter().for_each(|x| add(&x.get_ref().id));
        c.problem_set.iter().for_each(|x| add(&x.get_ref().id));
        c.word_problem.iter().for_each(|x| add(&x.id));
        c.hint.iter().for_each(|x| add(&x.id));
        c.praise.iter().for_each(|x| add(&x.id));
    }
    ids
}

/// A chapter's steps while they are being put in order.
struct Pending {
    skill: &'static Skill,
    /// (position in the file, which look it is for, the step).
    steps: Vec<(usize, LookTag, Step)>,
    /// (position in the file, the lines spoken there).
    lines: Vec<(usize, Vec<voice::Line>)>,
}

/// The content as it is built up, file by file.
#[derive(Default)]
struct Built {
    pending: Vec<Pending>,
    lessons: HashMap<(String, String), Vec<BeatDto>>,
    openers: HashMap<String, (String, String)>,
    sayings: HashMap<String, Sayings>,
    set_ends: HashMap<String, SetEndDto>,
}

impl Built {
    /// Checks and converts one chapter file, adding any errors to `errors`.
    fn add_chapter(&mut self, file: &str, chapter: &ChapterFile, errors: &mut Vec<ContentError>) {
        let Some(skill) = Skill::find(&chapter.skill) else {
            let message = format!("no skill called `{}`", chapter.skill);
            errors.push(ContentError::new(file, "", message));
            return;
        };
        if self.pending.iter().any(|p| p.skill.id == skill.id) {
            let message = format!("a second file for {}", skill.id);
            errors.push(ContentError::new(file, "", message));
            return;
        }
        // Report each message against the entry it came from.
        let mut report = |id: &str, messages: Vec<String>| {
            errors.extend(messages.iter().map(|m| ContentError::new(file, id, m)));
        };
        let mut steps = Vec::new();
        let mut lines = Vec::new();
        if let Some(first) = chapter.lesson.first() {
            let opener = (first.get_ref().id.clone(), chapter.title.clone());
            self.openers.insert(skill.id.to_string(), opener);
        }

        for lesson in &chapter.lesson {
            let (start, lesson) = (lesson.span().start, lesson.get_ref());
            match convert::lesson_beats(skill, lesson) {
                Ok(beats) => {
                    let spoken = beats.iter().filter_map(|b| b.voice.as_ref());
                    lines.push((start, spoken.flat_map(voice::lines_of).collect()));
                    let key = (skill.id.to_string(), lesson.id.clone());
                    self.lessons.insert(key, beats);
                    steps.push((start, lesson.look, Step::Lesson(lesson.id.clone())));
                }
                Err(messages) => report(&lesson.id, messages),
            }
        }
        for set in &chapter.problem_set {
            let (start, set) = (set.span().start, set.get_ref());
            match convert::problem_set(skill, set) {
                Ok(built) => {
                    lines.push((start, convert::set_lines(set)));
                    if let Some(end) = convert::set_end(set) {
                        self.set_ends.insert(set.id.clone(), end);
                    }
                    steps.push((start, set.look, Step::Set(built)));
                }
                Err(messages) => report(&set.id, messages),
            }
        }
        // What Ada says about answers. Their recordings are listed after the
        // chapter's lessons and sets (`usize::MAX` sorts them last, and the
        // sort keeps them in the order written).
        let mut sayings = Sayings::default();
        let mut spoken = Vec::new();
        for problem in &chapter.word_problem {
            report(&problem.id, check::check_word_problem(skill, problem));
            match convert::story(problem) {
                Ok(story) => {
                    spoken.extend(voice::lines_of(&story.note.voice));
                    sayings.stories.push(story);
                }
                Err(messages) => report(&problem.id, messages),
            }
        }
        for hint in &chapter.hint {
            match convert::hint(hint) {
                Ok(hint) => {
                    spoken.extend(voice::lines_of(&hint.note.voice));
                    sayings.hints.push(hint);
                }
                Err(messages) => report(&hint.id, messages),
            }
        }
        for praise in &chapter.praise {
            match convert::praise(praise) {
                Ok(praise) => {
                    spoken.extend(voice::lines_of(&praise.note.voice));
                    sayings.praise.push(praise);
                }
                Err(messages) => report(&praise.id, messages),
            }
        }
        lines.push((usize::MAX, spoken));
        self.sayings.insert(skill.id.to_string(), sayings);
        self.pending.push(Pending {
            skill,
            steps,
            lines,
        });
    }

    /// Every spoken line, chapter by chapter, each chapter in the order it
    /// was written in its file.
    fn lines(&self) -> Vec<voice::Line> {
        self.pending
            .iter()
            .flat_map(|p| {
                let mut lines: Vec<_> = p.lines.iter().collect();
                lines.sort_by_key(|(start, _)| *start);
                lines.into_iter().flat_map(|(_, spoken)| spoken.clone())
            })
            .collect()
    }

    /// The chapters a child in `look` sees, each with its steps back in the
    /// order they were written in the file.
    fn chapters(&self, look: Look) -> Vec<Chapter> {
        let wanted = |tag: LookTag| {
            matches!(
                (tag, look),
                (LookTag::Both, _)
                    | (LookTag::Gentle, Look::Gentle)
                    | (LookTag::Exceptional, Look::Exceptional)
            )
        };
        self.pending
            .iter()
            .map(|p| {
                let mut steps: Vec<_> = p.steps.iter().filter(|(_, tag, _)| wanted(*tag)).collect();
                steps.sort_by_key(|(start, _, _)| *start);
                Chapter {
                    skill: p.skill.id,
                    steps: steps.into_iter().map(|(_, _, step)| step.clone()).collect(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small chapter 1: lesson, set, a second lesson, a second set, written
    /// in that order, with one set only for the Exceptional look.
    const CH1: &str = r#"
skill = "count-on-within-5"
title = "The Next Number"

[[lesson]]
id = "first"
[[lesson.step]]
stage = "pictorial"
example = "3 + 1"
[[lesson.step.beat]]
text = "Three blocks make a tower."
show = "tower"
count = 3

[[problem_set]]
id = "walk"
stage = "pictorial"
pattern = "successor"
problems = ["0 + 1", "1 + 1", "2 + 1"]
notice = "Did you see? Each answer is the successor."

[[lesson]]
id = "second"
[[lesson.step]]
stage = "abstract"
example = "3 + 2"
[[lesson.step.beat]]
text = "Put in $3$ and see $f(3) = 4$."
formal = "Input $3$, output $f(3) = 4$."
show = "equation"
highlight = "answer"

[[problem_set]]
id = "grown-up-only"
stage = "abstract"
look = "exceptional"
problems = ["3 + 1", "3 + 2"]

[[word_problem]]
id = "stones"
stage = "pictorial"
text = "{a} stones and {b} more."
a = 3
b = 1

[[hint]]
id = "h1"
stage = "abstract"
mistake = "off-by-one"
text = "Count the hops."

[[praise]]
id = "p1"
text = "Good."
when = "adds-1"
"#;

    fn sources(files: &[(&str, &str)]) -> Vec<(String, String)> {
        files
            .iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect()
    }

    fn load(files: &[(&str, &str)]) -> Result<Content, Vec<ContentError>> {
        Content::from_sources(&sources(files))
    }

    /// The first error message, with its file and id, as the writer sees it.
    fn first_error(files: &[(&str, &str)]) -> String {
        load(files).err().expect("should be refused")[0].to_string()
    }

    fn ids(steps: &[Step]) -> Vec<String> {
        steps
            .iter()
            .map(|s| match s {
                Step::Lesson(id) => format!("lesson:{id}"),
                Step::Set(set) => format!("set:{}", set.id),
            })
            .collect()
    }

    #[test]
    fn steps_come_back_in_the_order_they_were_written() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        assert_eq!(
            ids(&content.chapters(Look::Exceptional)[0].steps),
            ["lesson:first", "set:walk", "lesson:second", "set:grown-up-only"]
        );
    }

    #[test]
    fn a_look_only_sees_its_own_entries() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        assert_eq!(
            ids(&content.chapters(Look::Gentle)[0].steps),
            ["lesson:first", "set:walk", "lesson:second"]
        );
    }

    #[test]
    fn a_written_set_carries_its_pattern_and_notice() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        let Step::Set(set) = &content.chapters(Look::Gentle)[0].steps[1] else {
            panic!("expected a set");
        };
        assert_eq!(set.skill, "count-on-within-5");
        assert_eq!(set.problems.len(), 3);
        assert_eq!(set.pattern, Some(primer_core::Pattern::Successor));
        assert_eq!(set.notice.as_deref(), Some("Did you see? Each answer is the successor."));
    }

    #[test]
    fn beats_carry_math_labels_and_highlights() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        let beats = content.beats("count-on-within-5", "second");
        assert_eq!(beats.len(), 1);
        let beat = &beats[0];
        assert!(beat.text.contains("<math"), "{}", beat.text);
        assert!(beat.formal.as_deref().unwrap().contains("<mi>f</mi>"));
        // The step's example, written out, because the beat has no label.
        assert_eq!(beat.label.as_deref(), Some("3 + 2 = 5"));
        assert_eq!(beat.highlight.as_deref(), Some("answer"));
    }

    #[test]
    fn machine_highlights_use_the_drawn_glow_names() {
        let file = CH1.replace(
            "show = \"equation\"\nhighlight = \"answer\"",
            "show = \"machine\"\nhighlight = \"input\"",
        );
        let content = load(&[("01-count-on.toml", &file)]).unwrap();
        let beat = &content.beats("count-on-within-5", "second")[0];
        assert_eq!(beat.highlight.as_deref(), Some("input-glow"));
        assert_eq!(beat.label, None);
    }

    #[test]
    fn a_swap_beat_lists_the_pouches_as_they_will_stand() {
        let file = CH1.replace(
            "show = \"equation\"\nhighlight = \"answer\"",
            "show = \"pouches\"\nleft = 5\nright = 3\nswap = true",
        );
        let content = load(&[("01-count-on.toml", &file)]).unwrap();
        let beat = &content.beats("count-on-within-5", "second")[0];
        assert_eq!((beat.left, beat.right), (Some(3), Some(5)));
    }

    #[test]
    fn a_typo_in_a_key_is_an_error_naming_the_file() {
        let file = CH1.replace("count = 3", "cuont = 3");
        let message = first_error(&[("01-count-on.toml", &file)]);
        assert!(message.starts_with("01-count-on.toml: "), "{message}");
        assert!(message.contains("cuont"), "{message}");
    }

    #[test]
    fn a_sum_over_the_limit_names_file_and_set() {
        let file = CH1.replace(r#"["0 + 1", "1 + 1", "2 + 1"]"#, r#"["0 + 1", "4 + 4"]"#);
        assert_eq!(
            first_error(&[("01-count-on.toml", &file)]),
            "01-count-on.toml: walk: `4 + 4` is outside what count-on-within-5 allows"
        );
    }

    #[test]
    fn an_unreadable_problem_is_an_error() {
        let file = CH1.replace(r#"["0 + 1", "1 + 1", "2 + 1"]"#, r#"["0 + 1", "one + one"]"#);
        assert!(first_error(&[("01-count-on.toml", &file)]).contains("`one + one` is not a problem"));
    }

    #[test]
    fn a_pattern_the_chapter_does_not_teach_is_an_error() {
        let file = CH1.replace(r#"pattern = "successor""#, r#"pattern = "bridge""#);
        assert!(first_error(&[("01-count-on.toml", &file)]).contains("walk: pattern `bridge`"));
    }

    #[test]
    fn broken_maths_is_an_error() {
        let file = CH1.replace("Count the hops.", "Count $3 + 4 hops.");
        assert!(first_error(&[("01-count-on.toml", &file)]).contains("h1: a `$` is never closed"));
    }

    #[test]
    fn an_unknown_skill_is_an_error() {
        let file = CH1.replace("count-on-within-5", "count-on-within-500");
        assert!(first_error(&[("01-count-on.toml", &file)]).contains("no skill called"));
    }

    #[test]
    fn an_id_used_in_two_files_is_an_error() {
        let other = CH1
            .replace("count-on-within-5", "add-within-10")
            .replace("\"first\"", "\"other-first\"");
        let message = first_error(&[("01-a.toml", CH1), ("02-b.toml", &other)]);
        assert!(message.ends_with("this id is used twice"), "{message}");
    }

    #[test]
    fn every_error_is_reported_not_just_the_first() {
        let file = CH1
            .replace(r#"["0 + 1", "1 + 1", "2 + 1"]"#, r#"["4 + 4"]"#)
            .replace("Count the hops.", "Count $ hops.");
        let errors = load(&[("01-count-on.toml", &file)]).err().unwrap();
        assert_eq!(errors.len(), 2, "{errors:?}");
    }

    #[test]
    fn words_and_other_files_are_handled_by_name() {
        let words = "[[word]]\nid = \"successor\"\nskill = \"count-on-within-5\"\ngentle = \"g\"\nformal = \"f\"\nada = \"a\"\n";
        let content = load(&[
            ("01-count-on.toml", CH1),
            ("_template.toml", "this is not toml at all"),
            ("hero_names.toml", "names = []"),
            ("words.toml", words),
        ])
        .unwrap();
        // `function` and `composition` are taught by chapter 1 but not listed.
        assert!(content.gaps.iter().any(|g| g == "words.toml: no entry for `function`"));
        assert!(!content.gaps.iter().any(|g| g.contains("`successor`")));
    }

    #[test]
    fn a_word_for_no_skill_is_an_error() {
        let words = "[[word]]\nid = \"x\"\nskill = \"nope\"\ngentle = \"g\"\nformal = \"f\"\nada = \"a\"\n";
        assert_eq!(first_error(&[("words.toml", words)]), "words.toml: x: no skill called `nope`");
    }

    #[test]
    fn the_gaps_say_what_is_not_written() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        let gaps = content.gaps.join("\n");
        assert!(gaps.contains("add-within-10: no chapter file yet"), "{gaps}");
        // The fixture has one hint, for the abstract stage's off-by-one.
        assert!(gaps.contains("01-count-on.toml: no hint for Pictorial/off-by-one"), "{gaps}");
        assert!(!gaps.contains("Abstract/off-by-one"), "{gaps}");
    }

    #[test]
    fn every_spoken_line_is_listed_in_book_order_with_its_name() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        let names: Vec<&str> = content.lines.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "first.1.1.gentle",
                "walk.notice.gentle",
                "second.1.1.gentle",
                "second.1.1.exceptional",
                // What Ada says about answers comes after the chapter's pages:
                // stories, then hints, then praise.
                "stones.gentle",
                "h1.gentle",
                "p1.gentle",
            ]
        );
        // The speech has the maths spelled out.
        assert_eq!(content.lines[3].speech, "Input 3, output f of 3 equals 4.");
        // The beat carries the same name for the page.
        let voice = content.beats("count-on-within-5", "second")[0].voice.clone().unwrap();
        assert_eq!(voice.gentle.name, "second.1.1.gentle");
    }

    #[test]
    fn only_a_chapters_first_lesson_has_an_opener() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        assert_eq!(
            content.opener("count-on-within-5", "first"),
            Some((1, "The Next Number"))
        );
        assert_eq!(content.opener("count-on-within-5", "second"), None);
        assert_eq!(content.opener("add-within-10", "first"), None);
    }

    #[test]
    fn an_unknown_lesson_has_no_beats() {
        let content = load(&[("01-count-on.toml", CH1)]).unwrap();
        assert!(content.beats("count-on-within-5", "nope").is_empty());
    }

    /// Every real chapter file in the repo's `content/` folder must load. The
    /// folder can hold only some of the files (or none) in a work branch, so
    /// this runs on whatever is there.
    #[test]
    fn every_file_in_the_content_folder_loads() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        if !dir.is_dir() {
            return;
        }
        if let Err(errors) = Content::load(&dir) {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            panic!("content does not load:\n{}", lines.join("\n"));
        }
    }
}
