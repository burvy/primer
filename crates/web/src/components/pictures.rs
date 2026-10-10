//! The pictures: counters, towers, ten frame, number line and the machines.
//!
//! Most are drawn here as SVG, because their size depends on the child's
//! numbers. Their colours come from CSS classes in `play.css`, which read the
//! look's variables, and their motion is CSS animation. The machines are
//! different: they are a hand-drawn file, and this code only loads it and
//! tells CSS which layer to light up.
//!
//! A Leptos *component* is a plain function that takes its inputs ("props")
//! and returns a view. Same props in, same picture out.

use leptos::prelude::*;
use primer_api::ProblemDto;

use crate::api;

/// Space between neighbouring counters and ticks, in SVG units.
const STEP: u32 = 30;
/// Size of one tower block, in SVG units.
const BLOCK_W: u32 = 64;
const BLOCK_H: u32 = 24;

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

/// `count` numbered blocks stacked from `base_y` upward, numbering from
/// `first + 1`. `class` picks the colour. Blocks with `arriving` set drop in
/// one after another (the delay grows with each block).
fn blocks(
    x: u32,
    first: u32,
    count: u32,
    base_y: u32,
    class: &'static str,
    arriving: bool,
) -> impl IntoView {
    (first..first + count)
        .map(|i| {
            let y = base_y - (i + 1) * BLOCK_H;
            let style = format!("animation-delay: {}ms", (i - first) * 300);
            view! {
                <g class=if arriving { "arriving" } else { "" } style=style>
                    <rect class=class x=x y=y width=BLOCK_W height=BLOCK_H />
                    <text class="block-num" x=x + BLOCK_W / 2 y=y + BLOCK_H - 7>{i + 1}</text>
                </g>
            }
        })
        .collect_view()
}

/// A tower of `count` blocks, with `adding` new blocks dropping onto the top.
/// Counting the blocks is counting on: 3, then 4.
#[component]
pub fn Tower(count: u32, adding: u32) -> impl IntoView {
    let height = (count + adding) * BLOCK_H + 10;
    view! {
        <svg class="picture tower" viewBox=format!("0 0 {} {height}", BLOCK_W + 20)>
            {blocks(10, 0, count, height - 5, "block-a", false)}
            {blocks(10, count, adding, height - 5, "block-b", true)}
        </svg>
    }
}

/// Coins per row inside a pouch's mouth, and on the table.
const IN_POUCH: u32 = 4;
const ON_TABLE: u32 = 5;
/// Height of the picture, and the line the pouches and coins rest on.
const SHEET_H: u32 = 200;
const TABLE_Y: u32 = 185;

/// One open pouch whose left edge is at `ox`, holding `count` coins.
///
/// The coins sit in the open mouth, in rows of four, so every coin can be
/// counted. An empty pouch is drawn slumped and flat with nothing in its
/// dark mouth, so zero looks like zero. `tilt` turns the pouch (degrees,
/// clockwise) about its mouth, which is how a pouch is shown pouring.
fn pouch(ox: u32, count: u32, tilt: i32) -> impl IntoView {
    let mouth_y = if count == 0 { 150 } else { 115 };
    let (left, right, mid) = (ox + 15, ox + 85, ox + 50);
    let body = format!(
        "M {left} {mouth_y} Q {} {TABLE_Y} {mid} {TABLE_Y} Q {} {TABLE_Y} {right} {mouth_y} Z",
        ox - 5,
        ox + 105
    );
    let coins = (0..count)
        .map(|k| {
            let (col, row) = (k % IN_POUCH, k / IN_POUCH);
            let (cx, cy) = (ox + 20 + col * 20, mouth_y - 8 - row * 20);
            view! { <circle class="coin" cx=cx cy=cy r=9 /> }
        })
        .collect_view();
    let turn = format!("rotate({tilt} {mid} {mouth_y})");
    view! {
        <g transform=turn>
            <path class="pouch-body" d=body />
            <ellipse class="pouch-mouth" cx=mid cy=mouth_y rx=35 ry=8 />
            {coins}
        </g>
    }
}

/// Two open coin pouches. Every coin is the same plain coin, worth 1.
///
/// - `join`: both pouches pour their coins out onto the table, where all the
///   coins lie together to be counted (they drop in one after another).
/// - `swap`: the pouches trade places. Each is drawn in its *final* place and
///   the CSS animation starts it from where it was (`--dx`), so the motion is
///   a slide into the finished picture.
/// - `total`: writes the total above.
#[component]
pub fn Pouches(left: u32, right: u32, join: bool, swap: bool, total: Option<u32>) -> impl IntoView {
    let width = 260;
    let (first, second) = (10, 150); // where the two pouches stand
    let label = total.map(|t| {
        view! { <text class="coin-total" x=width / 2 y=26>{format!("= {t}")}</text> }
    });
    let scene = if join {
        // Empty pouches tipped over the corners; all coins on the table.
        let pile = (0..left + right)
            .map(|k| {
                let (col, row) = (k % ON_TABLE, k / ON_TABLE);
                let style = format!("animation-delay: {}ms", k * 150);
                view! {
                    <g class="arriving" style=style>
                        <circle class="coin" cx=width / 2 - 40 + col * 20 cy=TABLE_Y - 10 - row * 20 r=9 />
                    </g>
                }
            })
            .collect_view();
        view! {
            <g transform="translate(0 -60)">{pouch(first, 0, 35)}</g>
            <g transform="translate(0 -60)">{pouch(second, 0, -35)}</g>
            {pile}
        }
        .into_any()
    } else {
        // After a swap the left pouch stands in the second place.
        let (left_at, right_at) = if swap {
            (second, first)
        } else {
            (first, second)
        };
        let travel = second as i32 - first as i32;
        let (left_dx, right_dx) = if swap { (-travel, travel) } else { (0, 0) };
        let classes = if swap { "slide" } else { "" };
        let moves = move |dx: i32| swap.then(|| format!("--dx: {dx}px"));
        view! {
            <g class=classes style=moves(left_dx)>{pouch(left_at, left, 0)}</g>
            <g class=classes style=moves(right_dx)>{pouch(right_at, right, 0)}</g>
        }
        .into_any()
    };
    view! {
        <svg class="picture pouches" viewBox=format!("0 10 {width} {}", SHEET_H - 10)>
            <line class="table" x1=0 y1=TABLE_Y x2=width y2=TABLE_Y />
            {scene}
            {label}
        </svg>
    }
}

/// What one cell of a ten frame holds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// A counter of the first kind.
    First,
    /// A counter of the second kind.
    Second,
    /// A counter taken away.
    Crossed,
    /// An empty dashed circle: the missing number the child must work out.
    Ghost,
}

/// The cells that picture a problem, in order, or none if it has no picture.
///
/// The hidden number is drawn as `Ghost` circles, never as counters, so the
/// picture helps without giving the answer away. `match` on a tuple looks at
/// the operation and the box position together.
pub fn cells(problem: &ProblemDto) -> Vec<Cell> {
    let (a, b) = (problem.a as usize, problem.b as usize);
    let row = |kind: Cell, n: usize| std::iter::repeat_n(kind, n);
    match (problem.op.as_str(), problem.blank.as_str()) {
        ("add", "answer") => row(Cell::First, a).chain(row(Cell::Second, b)).collect(),
        ("add", "second") => row(Cell::First, a).chain(row(Cell::Ghost, b)).collect(),
        ("add", "first") => row(Cell::Ghost, a).chain(row(Cell::Second, b)).collect(),
        ("sub", "answer") => row(Cell::First, a - b)
            .chain(row(Cell::Crossed, b))
            .collect(),
        ("sub", "second") => row(Cell::First, a - b).chain(row(Cell::Ghost, b)).collect(),
        _ => Vec::new(),
    }
}

/// Ten frames (5 columns, 2 rows), filled with `cells` in order. A new frame
/// starts after every ten.
#[component]
pub fn TenFrame(cells: Vec<Cell>) -> impl IntoView {
    let frames = cells.len().div_ceil(10).max(1);
    let one_frame = move |frame: usize| {
        let drawn = (0..10)
            .map(|slot| {
                let (x, y) = (slot % 5 * 40 + 5, slot / 5 * 40 + 5);
                let counter = cells.get(frame * 10 + slot).map(|cell| {
                    let class = match cell {
                        Cell::First => "counter-a",
                        Cell::Second => "counter-b",
                        Cell::Crossed => "counter-crossed",
                        Cell::Ghost => "counter-ghost",
                    };
                    view! { <circle class=class cx=x + 18 cy=y + 18 r=13 /> }
                });
                view! {
                    <rect class="cell" x=x y=y width=36 height=36 />
                    {counter}
                }
            })
            .collect_view();
        view! { <svg class="picture" viewBox="0 0 205 85">{drawn}</svg> }
    };
    (0..frames).map(one_frame).collect_view()
}

/// A number line from 0 to `max`, with a dot at `from`, `hop` one-step arcs
/// from there (forward when adding, backward when taking away), and an empty
/// ring at `target` when the child must find how to get there.
#[component]
pub fn NumberLine(
    max: u32,
    from: Option<u32>,
    hop: u32,
    forward: bool,
    #[prop(optional)] target: Option<u32>,
) -> impl IntoView {
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
    let ring = target.map(|goal| view! { <circle class="target" cx=x(goal) cy=40 r=9 /> });
    view! {
        <svg class="picture" viewBox=format!("0 0 {} 72", max * STEP + 30)>
            <line class="tick" x1=x(0) y1=40 x2=x(max) y2=40 />
            {ticks}
            {hops}
            {marker}
            {ring}
        </svg>
    }
}

/// Loads the hand-drawn machine once. Put the result in a signal so the
/// drawing appears as soon as it arrives.
fn load_drawing() -> RwSignal<String> {
    let drawing = RwSignal::new(String::new());
    leptos::task::spawn_local(async move {
        if let Ok(svg) = api::text("/assets/lessons/function-machine.svg").await {
            drawing.set(svg);
        }
    });
    drawing
}

/// One copy of the machine drawing. `glow` and `open` are written onto the
/// wrapper as `data-` attributes, and `play.css` uses them to show the matching
/// layer (`#input-glow`, `#output-glow`, `#hatch-open`). The code never draws
/// a glow.
#[component]
fn MachineDrawing(
    drawing: RwSignal<String>,
    glow: String,
    open: bool,
    class: &'static str,
) -> impl IntoView {
    view! {
        <div class=class
            data-glow=glow
            data-open=open.to_string()
            inner_html=move || drawing.get()></div>
    }
}

/// "in: 3 → out: 4", from whichever numbers are known.
fn caption(input: Option<u32>, output: Option<u32>) -> String {
    match (input, output) {
        (Some(i), Some(o)) => format!("in: {i}  →  out: {o}"),
        (Some(i), None) => format!("in: {i}"),
        _ => String::new(),
    }
}

/// Ada's machine.
#[component]
pub fn Machine(
    input: Option<u32>,
    output: Option<u32>,
    highlight: Option<String>,
    open: bool,
) -> impl IntoView {
    let drawing = load_drawing();
    view! {
        <MachineDrawing drawing glow=highlight.unwrap_or_default() open class="machine" />
        <p class="machine-io">{caption(input, output)}</p>
    }
}

/// Two machines joined by the first one's pipe: what comes out of machine 1
/// goes into machine 2 (one rule after another, called *composition*).
///
/// The second copy is placed so the end of the first copy's pipe lands on its
/// funnel; `play.css` holds the offsets and hides the second copy's own pipe.
/// The input glow belongs to machine 1 and the output glow to machine 2.
#[component]
pub fn Machines(
    input: Option<u32>,
    output: Option<u32>,
    highlight: Option<String>,
    open: bool,
) -> impl IntoView {
    let drawing = load_drawing();
    let glow = highlight.unwrap_or_default();
    let (first, second) = match glow.as_str() {
        "input-glow" => (glow.clone(), String::new()),
        "output-glow" => (String::new(), glow.clone()),
        _ => (String::new(), String::new()),
    };
    view! {
        <div class="machines">
            <MachineDrawing drawing glow=first open class="machine machine-1" />
            <MachineDrawing drawing glow=second open class="machine machine-2" />
        </div>
        <p class="machine-io">{caption(input, output)}</p>
    }
}
