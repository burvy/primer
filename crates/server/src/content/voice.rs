//! What Ada says aloud: a stable name for each spoken line, and the words to
//! read. Both are pure functions, so a voice actor's file list and the
//! browser's playback always agree.
//!
//! Every line has a `name`. A recording is found by that name, so it must
//! never change when the book is edited elsewhere: it is built from the
//! lesson's id and the line's position in it, not from the text.

use primer_api::{LineDto, VoiceDto};

/// The two looks a line can be read in.
pub const GENTLE: &str = "gentle";
pub const EXCEPTIONAL: &str = "exceptional";

/// One spoken line, as listed for the voice actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub name: String,
    /// `"gentle"` or `"exceptional"`.
    pub look: &'static str,
    /// The words to read aloud (see [`speakable`]).
    pub speech: String,
}

/// The name of a lesson beat's recording: `<lesson>.<step>.<beat>.<look>`.
///
/// `step` and `beat` count from 1, the way a person counts, so
/// `ch2-joining.1.2.gentle` is the second beat of the first step.
pub fn beat_name(lesson: &str, step: usize, beat: usize, look: &str) -> String {
    format!("{lesson}.{step}.{beat}.{look}")
}

/// The name of a set's recording: `<set>.<question|notice>.<look>`.
pub fn set_name(set: &str, part: &str, look: &str) -> String {
    format!("{set}.{part}.{look}")
}

/// The name of a hint's, praise's or story's recording: `<id>.<look>`.
pub fn note_name(id: &str, look: &str) -> String {
    format!("{id}.{look}")
}

/// The voice of a hint, a word of praise or a story. Like a beat's, the
/// Exceptional line exists only when there is a `formal` wording.
pub fn note_voice(id: &str, text: &str, formal: Option<&str>) -> VoiceDto {
    let line = |look: &str, text: &str| LineDto {
        name: note_name(id, look),
        speech: speakable(text),
    };
    VoiceDto {
        gentle: line(GENTLE, text),
        exceptional: formal.map(|formal| line(EXCEPTIONAL, formal)),
    }
}

/// The voice of one lesson beat. The Exceptional line exists only when the
/// beat has its own `formal` wording.
pub fn beat_voice(
    lesson: &str,
    step: usize,
    beat: usize,
    text: &str,
    formal: Option<&str>,
) -> VoiceDto {
    let line = |look: &str, text: &str| LineDto {
        name: beat_name(lesson, step, beat, look),
        speech: speakable(text),
    };
    VoiceDto {
        gentle: line(GENTLE, text),
        exceptional: formal.map(|formal| line(EXCEPTIONAL, formal)),
    }
}

/// The lines of a voice, for the recordings list.
pub fn lines_of(voice: &VoiceDto) -> Vec<Line> {
    let line = |look, dto: &LineDto| Line {
        name: dto.name.clone(),
        look,
        speech: dto.speech.clone(),
    };
    std::iter::once(line(GENTLE, &voice.gentle))
        .chain(voice.exceptional.iter().map(|dto| line(EXCEPTIONAL, dto)))
        .collect()
}

/// A sentence as it is read aloud: the words stay, each `$...$` of LaTeX is
/// spelled out (`$3 + 4 = 7$` becomes `3 plus 4 equals 7`).
///
/// `split('$')` cuts the text at each dollar sign, so every second piece is
/// maths. An unclosed `$` is already refused by the checks, so here the last
/// piece is simply read as words.
pub fn speakable(text: &str) -> String {
    let spoken: String = text
        .split('$')
        .enumerate()
        .map(|(index, piece)| {
            if index % 2 == 1 {
                format!(" {} ", maths_words(piece))
            } else {
                piece.to_string()
            }
        })
        .collect();
    // Squeeze the spaces, and pull a full stop back against its word.
    let squeezed = spoken.split_whitespace().collect::<Vec<_>>().join(" ");
    squeezed
        .replace(" .", ".")
        .replace(" ,", ",")
        .replace(" ?", "?")
        .replace(" !", "!")
        .replace(" :", ":")
}

/// LaTeX maths as words.
///
/// The text is read left to right, one symbol at a time, and each symbol
/// adds a word. The one rule that looks back: a bracket right after a single
/// letter is a function call, so it is read `of` (`f(3)` is `f of 3`,
/// `S(S(3))` is `S of S of 3`). Other brackets are not spoken.
fn maths_words(latex: &str) -> String {
    let chars: Vec<char> = latex.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        match c {
            '\\' => {
                // A command like `\mathbb{N}` or `\dots`: its name, then an
                // optional `{argument}`.
                let name: String = chars[i..].iter().take_while(|c| c.is_alphabetic()).collect();
                i += name.chars().count();
                let argument = braced(&chars, &mut i);
                match (name.as_str(), argument) {
                    ("dots" | "ldots" | "cdots", _) => words.push("and so on".into()),
                    ("times" | "cdot", _) => words.push("times".into()),
                    ("div", _) => words.push("divided by".into()),
                    ("le" | "leq", _) => words.push("is at most".into()),
                    ("ge" | "geq", _) => words.push("is at least".into()),
                    ("neq", _) => words.push("is not equal to".into()),
                    ("square" | "Box", _) => words.push("blank".into()),
                    // `\text{two}` is a word: read it as written.
                    ("text" | "mathrm" | "textbf" | "mathbf", Some(inside)) => words.push(inside),
                    // `\mathbb{N}` and friends: read the symbols inside.
                    (_, Some(inside)) => words.push(maths_words(&inside)),
                    (other, None) => words.push(other.to_string()),
                }
            }
            '+' => words.push("plus".into()),
            '-' | '−' => words.push("minus".into()),
            '=' => words.push("equals".into()),
            '<' => words.push("is less than".into()),
            '>' => words.push("is greater than".into()),
            '□' => words.push("blank".into()),
            '(' => {
                // After a lone letter, the bracket means "of".
                let after_letter = words.last().is_some_and(|w| {
                    w.chars().count() == 1 && w.chars().all(char::is_alphabetic)
                });
                if after_letter {
                    words.push("of".into());
                }
            }
            ',' | ':' | ';' => {
                if let Some(last) = words.last_mut() {
                    last.push(c);
                }
            }
            c if c.is_ascii_digit() => {
                let mut number = c.to_string();
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    number.push(chars[i]);
                    i += 1;
                }
                words.push(number);
            }
            c if c.is_alphabetic() => words.push(c.to_string()),
            // Spaces, closing brackets and braces are not spoken.
            _ => {}
        }
    }
    words.join(" ")
}

/// If a `{...}` follows at `*i`, returns what is inside and moves `*i` past
/// it. `&mut usize` lets this function move the caller's position.
fn braced(chars: &[char], i: &mut usize) -> Option<String> {
    if chars.get(*i) != Some(&'{') {
        return None;
    }
    let close = chars[*i..].iter().position(|&c| c == '}')? + *i;
    let inside: String = chars[*i + 1..close].iter().collect();
    *i = close + 1;
    Some(inside)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_count_from_one_and_carry_the_look() {
        assert_eq!(beat_name("ch2-joining", 1, 2, GENTLE), "ch2-joining.1.2.gentle");
        assert_eq!(set_name("ch2-swap-set", "notice", EXCEPTIONAL), "ch2-swap-set.notice.exceptional");
    }

    #[test]
    fn the_exceptional_line_exists_only_with_formal_wording() {
        let plain = beat_voice("l", 1, 1, "Hello.", None);
        assert_eq!(plain.exceptional, None);
        let both = beat_voice("l", 2, 3, "Hello.", Some("Greetings, $a$."));
        assert_eq!(both.gentle.name, "l.2.3.gentle");
        let formal = both.exceptional.unwrap();
        assert_eq!(formal.name, "l.2.3.exceptional");
        assert_eq!(formal.speech, "Greetings, a.");
    }

    #[test]
    fn lines_of_lists_each_look() {
        let voice = beat_voice("l", 1, 1, "One.", Some("Uno."));
        let lines = lines_of(&voice);
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].look, lines[1].look), (GENTLE, EXCEPTIONAL));
    }

    #[test]
    fn words_are_left_alone() {
        assert_eq!(speakable("Put in 3. Out comes 4!"), "Put in 3. Out comes 4!");
    }

    #[test]
    fn sums_are_spelled_out() {
        assert_eq!(speakable("We write $3 + 4 = 7$. Good."), "We write 3 plus 4 equals 7. Good.");
        assert_eq!(speakable("$9 - 2 = 7$"), "9 minus 2 equals 7");
    }

    #[test]
    fn brackets_after_a_letter_are_read_of() {
        assert_eq!(speakable("$S(3) = 4$"), "S of 3 equals 4");
        assert_eq!(speakable("$f(3) = 4$"), "f of 3 equals 4");
        assert_eq!(speakable("$S(S(3)) = 5$"), "S of S of 3 equals 5");
        assert_eq!(speakable("$f(x) = x + 1$"), "f of x equals x plus 1");
    }

    #[test]
    fn plain_brackets_are_silent() {
        assert_eq!(speakable("$(3 + 4) = 7$"), "3 plus 4 equals 7");
    }

    #[test]
    fn symbols_have_words() {
        assert_eq!(speakable(r"in $\mathbb{N}$"), "in N");
        assert_eq!(
            speakable(r"the numbers $0, 1, 2, 3, \dots$ form a set"),
            "the numbers 0, 1, 2, 3, and so on form a set"
        );
        assert_eq!(speakable(r"$a + b = b + a$"), "a plus b equals b plus a");
        assert_eq!(speakable(r"$3 + \square = 7$"), "3 plus blank equals 7");
        assert_eq!(speakable(r"$\text{two}$"), "two");
    }

    #[test]
    fn a_colon_stays_against_its_word() {
        assert_eq!(
            speakable(r"For every $a$ and $b$ in $\mathbb{N}$: $a + b = b + a$."),
            "For every a and b in N: a plus b equals b plus a."
        );
    }
}
