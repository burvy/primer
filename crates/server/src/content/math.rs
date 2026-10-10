//! Maths in text: `$...$` is LaTeX, and the page shows it as MathML.
//!
//! MathML is the web's own markup for maths (`<math>`, `<mi>`, `<mo>`...).
//! Browsers draw it natively, in the maths font, so the server only has to
//! translate. [`to_html`] turns a whole sentence into a piece of HTML:
//! the words are escaped (so a `<` in a sentence can never become a tag) and
//! each `$...$` becomes a `<math>` element.

use latex2mathml::{DisplayStyle, latex_to_mathml};

/// `text` as HTML, with every `$...$` turned into inline MathML.
///
/// `split('$')` cuts the text at each dollar sign, so the pieces alternate:
/// words, maths, words, maths, ... Every odd piece is maths. An even number of
/// pieces means a `$` was never closed, which is an error the writer must fix.
///
/// `Result<String, String>`: `Ok` holds the HTML, `Err` holds a message for
/// the writer (the caller adds the file and id).
pub fn to_html(text: &str) -> Result<String, String> {
    let pieces: Vec<&str> = text.split('$').collect();
    if pieces.len().is_multiple_of(2) {
        return Err(format!("a `$` is never closed in: {text}"));
    }
    let mut html = String::new();
    for (index, piece) in pieces.iter().enumerate() {
        if index % 2 == 1 {
            let mathml = latex_to_mathml(piece, DisplayStyle::Inline)
                .map_err(|error| format!("cannot read the maths `{piece}`: {error}"))?;
            html.push_str(&mathml);
        } else {
            html.push_str(&escape(piece));
        }
    }
    Ok(html)
}

/// Makes plain words safe to place inside HTML.
fn escape(words: &str) -> String {
    words
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_words_stay_words() {
        assert_eq!(to_html("Put in 3.").unwrap(), "Put in 3.");
    }

    #[test]
    fn words_are_escaped() {
        assert_eq!(to_html("2 < 3 & 4 > 1").unwrap(), "2 &lt; 3 &amp; 4 &gt; 1");
    }

    #[test]
    fn maths_becomes_mathml_and_words_stay_around_it() {
        let html = to_html("So $f(3) = 4$ here.").unwrap();
        assert!(html.starts_with("So <math"), "{html}");
        assert!(html.contains("<mi>f</mi>"), "{html}");
        assert!(html.ends_with("</math> here."), "{html}");
    }

    #[test]
    fn the_writers_symbols_all_convert() {
        for latex in [
            r"\mathbb{N}",
            r"0, 1, 2, 3, \dots",
            "S(S(3))",
            "f(3)",
            "a + b = b + a",
            r"For every $a$ in $\mathbb{N}$: $a + 0 = a$",
        ] {
            let text = format!("${}$", latex.replace('$', ""));
            let html = to_html(&text).unwrap();
            assert!(html.contains("<math"), "{latex} gave {html}");
        }
    }

    #[test]
    fn an_unclosed_dollar_is_an_error() {
        assert!(to_html("Cost $3 + 4").is_err());
    }

    #[test]
    fn unreadable_maths_is_an_error() {
        assert!(to_html(r"$\frac{1$").is_err());
    }
}
