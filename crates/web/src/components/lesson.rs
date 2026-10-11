//! A lesson: a list of beats, shown one tap at a time.

use leptos::prelude::*;
use primer_api::BeatDto;
use primer_core::Look;

use crate::art;
use crate::audio;

use super::buttons::Turn;
use super::equation::{LabelEquation, RichText};
use super::pictures::{Counters, Machine, Machines, NumberLine, Pouches, Tower};

/// Which machines have been fed what, as of beat `at`, and which machine runs
/// in that beat. A pure function of the lesson's beats, so it is tested here.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MachineProgress {
    /// (input, output) of each machine fed so far in the current run of
    /// machine beats. A single `machine` keeps only its latest numbers.
    pub fed: Vec<(u32, Option<u32>)>,
    /// The machine that runs in beat `at` (0 is the first), if any.
    pub running: Option<usize>,
}

/// Works out [`MachineProgress`] for beat `at`.
///
/// Walk back from `at` over the beats that show the same kind of picture
/// (`machine` or `machines`); that run of beats is one machine scene. Each
/// beat in it that puts a number in is one machine fed: for `machines` the
/// first such beat is machine 0, the next is machine 1, and so on. The
/// numbers stay for the later beats of the scene.
pub fn machine_progress(beats: &[BeatDto], at: usize) -> MachineProgress {
    let Some(now) = beats.get(at) else {
        return MachineProgress::default();
    };
    if now.show != "machine" && now.show != "machines" {
        return MachineProgress::default();
    }
    let start = (0..=at)
        .rev()
        .take_while(|&i| beats[i].show == now.show)
        .last()
        .unwrap_or(at);
    let fed: Vec<(u32, Option<u32>)> = beats[start..=at]
        .iter()
        .filter_map(|b| b.input.map(|input| (input, b.output)))
        .collect();
    let running = now.input.map(|_| match now.show.as_str() {
        "machines" => fed.len() - 1,
        _ => 0,
    });
    let fed = match now.show.as_str() {
        "machine" => fed.last().copied().into_iter().collect(),
        _ => fed,
    };
    MachineProgress { fed, running }
}

/// The lines written under the machines: "in: 3 → out: 4". With several
/// machines each line says which machine it is.
pub fn machine_lines(fed: &[(u32, Option<u32>)], joined: bool) -> Vec<String> {
    fed.iter()
        .enumerate()
        .map(|(k, &(input, output))| {
            let numbers = match output {
                Some(output) => format!("in: {input}  →  out: {output}"),
                None => format!("in: {input}"),
            };
            if joined {
                format!("machine {}: {numbers}", k + 1)
            } else {
                numbers
            }
        })
        .collect()
}

/// The picture for one beat. `match` on the `show` word picks the drawing;
/// the beat's small settings (`count`, `at`, ...) are handed to it.
/// `running` is the machine that moves in this beat (see [`machine_progress`]).
fn number_line_settings(beat: &BeatDto) -> (u32, bool) {
    let (at, hop) = (beat.at.unwrap_or(0), beat.hop.unwrap_or(0));
    let back = beat.back.unwrap_or(false);
    let max = if back { at.max(5) } else { (at + hop).max(5) };
    (max, !back)
}

#[component]
fn Picture(beat: BeatDto, running: Option<usize>) -> impl IntoView {
    match beat.show.as_str() {
        "counters" => {
            view! { <Counters count=beat.count.unwrap_or(0) adding=beat.adding.unwrap_or(0) /> }
                .into_any()
        }
        "number-line" => {
            let hop = beat.hop.unwrap_or(0);
            let (max, forward) = number_line_settings(&beat);
            view! { <NumberLine max=max from=beat.at hop=hop forward=forward /> }
                .into_any()
        }
        "machine" => view! {
            <Machine
                input=beat.input
                highlight=beat.highlight.clone()
                open=beat.open.unwrap_or(false)
                label=beat.label.clone()
            />
        }
        .into_any(),
        "machines" => view! {
            <Machines
                running=running
                highlight=beat.highlight.clone()
                open=beat.open.unwrap_or(false)
                label=beat.label.clone()
            />
        }
        .into_any(),
        "tower" => {
            view! { <Tower count=beat.count.unwrap_or(0) adding=beat.adding.unwrap_or(0) /> }
                .into_any()
        }
        "pouches" => view! {
            <Pouches
                left=beat.left.unwrap_or(0)
                right=beat.right.unwrap_or(0)
                join=beat.join.unwrap_or(false)
                swap=beat.swap.unwrap_or(false)
                total=beat.total
            />
        }
        .into_any(),
        "equation" => view! {
            <LabelEquation
                label=beat.label.clone().unwrap_or_default()
                highlight=beat.highlight.clone()
            />
        }
        .into_any(),
        other => view! { <p class="note">{format!("[picture \"{other}\" not drawn yet]")}</p> }
            .into_any(),
    }
}

/// Ada, the narrator. The hand-drawn `assets/lessons/ada.svg` if the file
/// exists, else the silhouette from the stylesheet. In the drawing, `play.css`
/// blinks her (`eyes-open` / `eyes-closed`) and opens her mouth (`mouth-open`)
/// while `<body>` has the class `speaking`. Tapping her reads the line again.
#[component]
pub fn Ada(#[prop(into)] on_tap: Callback<()>) -> impl IntoView {
    let art = art::drawing("lessons/ada.svg");
    view! {
        <div class="ada" class:has-art=move || art.get().is_some()
            role="button" tabindex="0"
            aria-label="Ada, the narrator. Tap to hear her again."
            inner_html=move || art.get().unwrap_or_default()
            on:click=move |_| on_tap.run(())></div>
    }
}

/// Shows beat number `step` of `beats`. Tapping the button moves on; after the
/// last beat it calls `on_done`. The Exceptional look shows each beat's
/// `formal` wording instead of `text` when it has one.
#[component]
pub fn LessonPage(
    beats: Vec<BeatDto>,
    look: RwSignal<Look>,
    #[prop(into)] on_done: Callback<()>,
) -> impl IntoView {
    let step = RwSignal::new(0usize);
    let last = beats.len().saturating_sub(1);
    let beats = StoredValue::new(beats);

    // Ada reads beat `at` aloud, in the line that fits the look: the
    // `exceptional` one when the beat has it, else the `gentle` one.
    let read_aloud = move |at: usize, look: Look| {
        let Some(voice) = beats.with_value(|all| all.get(at).and_then(|b| b.voice.clone())) else {
            return;
        };
        let line = match (look, voice.exceptional) {
            (Look::Exceptional, Some(formal)) => formal,
            _ => voice.gentle,
        };
        audio::say(&line.name, &line.speech);
    };
    // Each new beat is read, and gets its sound if its picture has one.
    // The Effect re-runs when the beat or the look changes.
    Effect::new(move |_| {
        let at = step.get();
        read_aloud(at, look.get());
        let beat = beats.with_value(|all| all.get(at).cloned()).unwrap_or_default();
        match beat.show.as_str() {
            "pouches" if beat.join == Some(true) => audio::sound("coin"),
            "number-line" if beat.hop.unwrap_or(0) > 0 => audio::sound("hop"),
            _ => {}
        }
    });
    // Leaving the lesson stops Ada mid-sentence.
    on_cleanup(audio::stop);

    view! {
        <div class="lesson">
            // The picture. `leaf` plays the page-turn each time a new beat is drawn.
            {move || {
                let beat = beats.with_value(|all| all.get(step.get()).cloned()).unwrap_or_default();
                let running = beats.with_value(|all| machine_progress(all, step.get()).running);
                view! {
                    <div class="leaf">
                        <Picture beat=beat running=running />
                    </div>
                }
            }}
            // What was fed to the machines. It is not part of the page-turn,
            // so the numbers stay put once shown, and grow as machines are fed.
            <div class="machine-numbers">
                {move || {
                    let (progress, joined) = beats.with_value(|all| {
                        let joined = all.get(step.get()).is_some_and(|b| b.show == "machines");
                        (machine_progress(all, step.get()), joined)
                    });
                    machine_lines(&progress.fed, joined)
                        .into_iter()
                        .map(|line| view! { <p class="machine-io">{line}</p> })
                        .collect_view()
                }}
            </div>
            {move || {
                let beat = beats.with_value(|all| all.get(step.get()).cloned()).unwrap_or_default();
                let text = match (look.get(), &beat.formal) {
                    (Look::Exceptional, Some(formal)) => formal.clone(),
                    _ => beat.text.clone(),
                };
                // Ada beside her line, as on the practice pages.
                view! {
                    <div class="leaf says">
                        <Ada on_tap=move |()| read_aloud(step.get_untracked(), look.get_untracked()) />
                        <p class="said"><RichText text=text /></p>
                    </div>
                }
            }}
            <Turn on_click=move |()| {
                audio::sound("page-turn");
                if step.get() >= last {
                    on_done.run(());
                } else {
                    step.update(|s| *s += 1);
                }
            }>
                {move || if step.get() >= last { "Done" } else { "Next ▸" }}
            </Turn>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beat(show: &str, input: Option<u32>, output: Option<u32>) -> BeatDto {
        BeatDto {
            show: show.to_string(),
            input,
            output,
            ..BeatDto::default()
        }
    }

    /// The lesson of two joined machines: introduce, feed the first, feed the
    /// second, then name the joined rule.
    fn two_machines() -> Vec<BeatDto> {
        vec![
            beat("machines", None, None),
            beat("machines", Some(3), Some(4)),
            beat("machines", Some(4), Some(5)),
            beat("machines", None, None),
        ]
    }

    #[test]
    fn each_joined_machine_runs_only_on_its_own_beat() {
        let beats = two_machines();
        let running: Vec<Option<usize>> =
            (0..4).map(|at| machine_progress(&beats, at).running).collect();
        assert_eq!(running, [None, Some(0), Some(1), None]);
    }

    #[test]
    fn the_numbers_stay_once_shown() {
        let beats = two_machines();
        let fed = |at| machine_progress(&beats, at).fed;
        assert_eq!(fed(0), []);
        assert_eq!(fed(1), [(3, Some(4))]);
        assert_eq!(fed(2), [(3, Some(4)), (4, Some(5))]);
        // The last beat has no number of its own, and still shows both.
        assert_eq!(fed(3), [(3, Some(4)), (4, Some(5))]);
    }

    #[test]
    fn a_single_machine_keeps_its_latest_numbers() {
        let beats = vec![
            beat("machine", Some(3), Some(4)),
            beat("machine", None, None),
            beat("machine", Some(5), Some(6)),
        ];
        let fed: Vec<_> = (0..3).map(|at| machine_progress(&beats, at).fed).collect();
        assert_eq!(fed[0], [(3, Some(4))]);
        assert_eq!(fed[1], [(3, Some(4))]);
        assert_eq!(fed[2], [(5, Some(6))]);
        assert_eq!(machine_progress(&beats, 0).running, Some(0));
        assert_eq!(machine_progress(&beats, 1).running, None);
    }

    #[test]
    fn another_picture_clears_the_numbers_and_a_new_scene_starts_again() {
        let beats = vec![
            beat("machine", Some(3), Some(4)),
            beat("tower", None, None),
            beat("machines", Some(1), Some(2)),
        ];
        assert_eq!(machine_progress(&beats, 1), MachineProgress::default());
        // The second scene has a different picture, so it starts from machine 0.
        assert_eq!(machine_progress(&beats, 2).fed, [(1, Some(2))]);
        assert_eq!(machine_progress(&beats, 2).running, Some(0));
        assert_eq!(machine_progress(&beats, 9), MachineProgress::default());
    }

    #[test]
    fn the_lines_name_the_machine_when_there_are_several() {
        let fed = [(3, Some(4)), (4, None)];
        assert_eq!(machine_lines(&fed, false), ["in: 3  →  out: 4", "in: 4"]);
        assert_eq!(
            machine_lines(&fed, true),
            ["machine 1: in: 3  →  out: 4", "machine 2: in: 4"]
        );
    }

    #[test]
    fn number_line_defaults_to_forward_hops() {
        let beat = BeatDto {
            at: Some(4),
            hop: Some(3),
            ..BeatDto::default()
        };
        assert_eq!(number_line_settings(&beat), (7, true));
    }
}
