//! The pictures: counters, ten frame, number line and the machine.
//!
//! The first three are drawn here as SVG, because their size depends on the
//! child's numbers. Their colours come from CSS classes in `play.css`, which
//! read the look's variables. The machine is different: it is a hand-drawn
//! file, and this code only loads it and tells CSS which layer to light up.
//!
//! A Leptos *component* is a plain function that takes its inputs ("props")
//! and returns a view. Same props in, same picture out.

use leptos::prelude::*;

use crate::api;

/// Space between neighbouring counters and ticks, in SVG units.
const STEP: u32 = 30;

/// A row of counters: `count` of the first kind, then `adding` of the second.
#[component]
pub fn Counters(count: u32, adding: u32) -> impl IntoView {
    let width = (count + adding) * STEP + 10;
    let dots = (0..count + adding)
        .map(|i| {
            let class = if i < count { "counter-a" } else { "counter-b" };
            view! { <circle class=class cx=i * STEP + 25 cy=25 r=11 /> }
        })
        .collect_view();
    view! {
        <svg class="picture" viewBox=format!("0 0 {width} 50")>{dots}</svg>
    }
}

/// Ten frames (5 columns, 2 rows) holding `a` counters and then `b` more.
/// When `take_away` is true the last `b` counters are crossed out instead.
#[component]
pub fn TenFrame(a: u32, b: u32, take_away: bool) -> impl IntoView {
    let total = if take_away { a } else { a + b };
    let frames = total.div_ceil(10).max(1);
    let one_frame = move |frame: u32| {
        let cells = (0..10)
            .map(|cell| {
                let n = frame * 10 + cell; // which counter this cell would hold
                let (x, y) = (cell % 5 * 40 + 5, cell / 5 * 40 + 5);
                let counter = (n < total).then(|| {
                    let second = if take_away { n >= a - b } else { n >= a };
                    let class = match (second, take_away) {
                        (false, _) => "counter-a",
                        (true, false) => "counter-b",
                        (true, true) => "counter-crossed",
                    };
                    view! { <circle class=class cx=x + 18 cy=y + 18 r=13 /> }
                });
                view! {
                    <rect class="cell" x=x y=y width=36 height=36 />
                    {counter}
                }
            })
            .collect_view();
        view! { <svg class="picture" viewBox="0 0 205 85">{cells}</svg> }
    };
    (0..frames).map(one_frame).collect_view()
}

/// A number line from 0 to `max`, with a dot at `from` and `hop` one-step
/// arcs from there, forward (adding) or backward (taking away).
#[component]
pub fn NumberLine(max: u32, from: Option<u32>, hop: u32, forward: bool) -> impl IntoView {
    let x = move |n: u32| n * STEP + 15;
    let ticks = (0..=max)
        .map(|n| {
            view! {
                <line class="tick" x1=x(n) y1=40 x2=x(n) y2=48 />
                <text class="label" x=x(n) y=64>{n}</text>
            }
        })
        .collect_view();
    let hops = from.map(|start| {
        (0..hop)
            .map(|i| {
                let (here, there) = if forward {
                    (start + i, start + i + 1)
                } else {
                    (start - i, start - i - 1)
                };
                let path = format!(
                    "M {} 40 Q {} 8 {} 40",
                    x(here),
                    (x(here) + x(there)) / 2,
                    x(there)
                );
                view! { <path class="hop" d=path /> }
            })
            .collect_view()
    });
    let marker = from.map(|start| view! { <circle class="marker" cx=x(start) cy=40 r=5 /> });
    view! {
        <svg class="picture" viewBox=format!("0 0 {} 72", max * STEP + 30)>
            <line class="tick" x1=x(0) y1=40 x2=x(max) y2=40 />
            {ticks}
            {hops}
            {marker}
        </svg>
    }
}

/// Ada's machine. The drawing is a file made by hand; we load it once and put
/// it on the page. `highlight` and `open` are written onto the wrapper as
/// `data-` attributes, and `play.css` uses them to show the matching layer
/// (`#input-glow`, `#output-glow`, `#hatch-open`). The code never draws a glow.
#[component]
pub fn Machine(
    input: Option<u32>,
    output: Option<u32>,
    highlight: Option<String>,
    open: bool,
) -> impl IntoView {
    let drawing = RwSignal::new(String::new());
    leptos::task::spawn_local(async move {
        if let Ok(svg) = api::text("/assets/lessons/function-machine.svg").await {
            drawing.set(svg);
        }
    });
    let caption = match (input, output) {
        (Some(i), Some(o)) => format!("in: {i}  →  out: {o}"),
        (Some(i), None) => format!("in: {i}"),
        _ => String::new(),
    };
    view! {
        <div class="machine"
            data-glow=highlight.unwrap_or_default()
            data-open=open.to_string()
            inner_html=move || drawing.get()></div>
        <p class="machine-io">{caption}</p>
    }
}
