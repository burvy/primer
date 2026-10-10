//! `primer-lint`: checks the book's content files and prints a short report.
//!
//! ```text
//! primer-lint                 code checks only (free, no network)
//! primer-lint --jev           also ask Jev about every line (cached)
//! primer-lint draft           later: drafting help for the writer
//! ```
//!
//! Code checks are the server's own: the same loader that refuses broken
//! content at start-up, reused from `primer_server::content`, plus the word
//! limits from the writer's guide. Jev checks run only with `--jev`, and each
//! line's verdict is cached in `.lint-cache.json` by a hash of its text, so
//! an unchanged line never costs a second call.
//!
//! The exit code is 0 when nothing was flagged and 1 otherwise, so a script
//! can stop on a bad file.

mod entries;
mod review;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use primer_server::config::JevConfig;
use primer_server::content::Content;
use primer_server::content::schema::ChapterFile;
use primer_server::jev::JevClient;

use crate::review::{Ask, Cache};

/// `clap` reads the command line into this struct. Each field is one option;
/// the doc comments become the `--help` text.
#[derive(Parser)]
#[command(name = "primer-lint", about = "Checks Primer's content files")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// The folder of content files.
    #[arg(long, default_value = "content")]
    content: PathBuf,
    /// Also ask Jev about each line (needs the key file; see PRIMER_JEV_KEY_FILE).
    #[arg(long)]
    jev: bool,
    /// Where Jev's verdicts are remembered between runs.
    #[arg(long, default_value = ".lint-cache.json")]
    cache: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Later: a strong model drafts lines for the human to rewrite.
    Draft,
}

/// What one run found.
#[derive(Debug, Default)]
struct Report {
    /// For each file, the lines to show: `ch2-swap question: gives the answer away`.
    files: BTreeMap<String, Vec<String>>,
    /// What is not written yet: information, not a failure.
    gaps: Vec<String>,
    /// Lines checked (each plain or formal wording counts once).
    lines: usize,
    /// Lines Jev could not check this time, and the first reason.
    unchecked: usize,
    first_error: Option<String>,
}

impl Report {
    fn flagged(&self) -> usize {
        self.files.values().map(Vec::len).sum()
    }

    fn flag(&mut self, file: &str, line: String) {
        self.files.entry(file.to_string()).or_default().push(line);
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Some(Command::Draft) = cli.command {
        println!(
            "primer-lint draft: not built yet (later: a strong model drafts, the human rewrites)."
        );
        return ExitCode::SUCCESS;
    }
    // A small async engine on this one thread; Jev calls are made one by one.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    match runtime
        .map_err(|e| e.to_string())
        .and_then(|rt| rt.block_on(run(&cli)))
    {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("primer-lint: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The edge: read files, lint, print, save the cache. `Ok(true)` means clean.
async fn run(cli: &Cli) -> Result<bool, String> {
    let sources = read_sources(&cli.content)?;
    let report = if cli.jev {
        let mut config =
            JevConfig::from_env().ok_or("--jev needs the key file named by PRIMER_JEV_KEY_FILE")?;
        // The server's 2 s is for a waiting child; a writer can wait longer.
        config.timeout = Duration::from_secs(20);
        let client = JevClient::new(config).map_err(|e| e.to_string())?;
        let mut cache = load_cache(&cli.cache);
        let report = lint(&sources, Some(&client), &mut cache).await;
        let text = serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?;
        std::fs::write(&cli.cache, text).map_err(|e| format!("{}: {e}", cli.cache.display()))?;
        report
    } else {
        lint::<JevClient>(&sources, None, &mut Cache::new()).await
    };
    print!("{}", render(&report, cli.jev));
    Ok(report.flagged() == 0)
}

/// Every `.toml` file in `dir`, in name order, as `(name, text)`.
fn read_sources(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let fail = |e: std::io::Error| format!("{}: {e}", dir.display());
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(fail)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            std::fs::read_to_string(&path)
                .map(|text| (name, text))
                .map_err(fail)
        })
        .collect()
}

/// A missing or unreadable cache is just an empty one.
fn load_cache(path: &Path) -> Cache {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The whole lint over `(file name, text)` pairs. With `jev = None` only the
/// code checks run. Generic over `A: Ask`, so tests pass a fake Jev.
async fn lint<A: Ask>(sources: &[(String, String)], jev: Option<&A>, cache: &mut Cache) -> Report {
    let mut report = Report::default();
    // 1. The server's loader and checks, exactly as at start-up.
    match Content::from_sources(sources) {
        Ok(content) => report.gaps = content.gaps,
        Err(errors) => {
            for error in errors {
                let place = if error.id.is_empty() {
                    String::new()
                } else {
                    format!("{}: ", error.id)
                };
                report.flag(&error.file, format!("{place}{}", error.message));
            }
        }
    }
    // 2. Line by line: word limits, then Jev if asked.
    for (file, text) in sources
        .iter()
        .filter(|(name, _)| name.starts_with(|c: char| c.is_ascii_digit()))
    {
        // A file that does not parse was reported in step 1.
        let Ok(chapter) = toml::from_str::<ChapterFile>(text) else {
            continue;
        };
        for entry in entries::entries(&chapter) {
            report.lines += 1;
            if let Some(problem) = entries::word_limit(&entry) {
                report.flag(file, format!("{}: {problem}", entry.place));
            }
            let Some(jev) = jev else { continue };
            match review::review(jev, &entry, cache).await {
                Ok(verdict) => {
                    for problem in review::findings(&entry, &verdict) {
                        report.flag(file, format!("{}: {problem}", entry.place));
                    }
                }
                Err(error) => {
                    report.unchecked += 1;
                    report.first_error.get_or_insert(error.to_string());
                }
            }
        }
    }
    report
}

/// The report as the writer reads it.
fn render(report: &Report, jev: bool) -> String {
    let mut out = String::new();
    for (file, lines) in &report.files {
        out.push_str(&format!("{file}\n"));
        for line in lines {
            out.push_str(&format!("  {line}\n"));
        }
    }
    for gap in &report.gaps {
        out.push_str(&format!("not written yet: {gap}\n"));
    }
    if let Some(error) = &report.first_error {
        out.push_str(&format!(
            "jev: {} lines not checked ({error}); run again to retry\n",
            report.unchecked
        ));
    }
    let checks = if jev {
        "code and Jev checks"
    } else {
        "code checks only; add --jev for Jev checks"
    };
    out.push_str(&format!(
        "primer-lint: {} lines, {} flagged ({checks})\n",
        report.lines,
        report.flagged()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::tests::FakeJev;

    /// The human's real chapters are the test corpus.
    fn real_content() -> Vec<(String, String)> {
        read_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
    }

    #[tokio::test]
    async fn the_real_chapters_pass_the_servers_checks() {
        let report = lint::<JevClient>(&real_content(), None, &mut Cache::new()).await;
        assert!(report.lines > 50, "found {} lines", report.lines);
        // Only word limits may be flagged here: the loader itself must pass.
        for line in report.files.values().flatten() {
            assert!(line.contains("words; keep it under"), "{line}");
        }
    }

    #[tokio::test]
    async fn a_second_run_makes_no_jev_calls() {
        let (fake, mut cache) = (FakeJev::default(), Cache::new());
        let sources = real_content();
        let first = lint(&sources, Some(&fake), &mut cache).await;
        let calls = fake.calls.get();
        assert_eq!(calls, first.lines, "one call per line");
        let second = lint(&sources, Some(&fake), &mut cache).await;
        assert_eq!(fake.calls.get(), calls, "the re-run is free");
        assert_eq!(first.flagged(), second.flagged());
    }

    #[tokio::test]
    async fn broken_content_is_reported_by_file_and_id() {
        let sources = vec![(
            "09-x.toml".to_string(),
            "skill = \"no-such-skill\"\ntitle = \"X\"\n".to_string(),
        )];
        let report = lint::<JevClient>(&sources, None, &mut Cache::new()).await;
        assert!(report.files.contains_key("09-x.toml"), "{report:?}");
        assert!(render(&report, false).starts_with("09-x.toml\n  "));
    }
}
