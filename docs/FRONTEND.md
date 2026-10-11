# How the pages are built (a note for the human)

You do not need to know web design to read this. It explains, in the order
you will meet them, the five things that make the screen: **components**,
**signals**, **CSS**, **layout**, and **the drawings**. Then how to try a
change and see it at once.

Everything lives in `crates/web/`. The files named below, to open:
[style.css](../crates/web/style.css), [play.css](../crates/web/play.css),
[index.html](../crates/web/index.html), [pages/](../crates/web/src/pages/),
[components/](../crates/web/src/components/), [art.rs](../crates/web/src/art.rs),
[PALETTE.md](PALETTE.md), [ART_GUIDE.md](ART_GUIDE.md).

```
crates/web/
  index.html          the one page; loads the two stylesheets and the scripts
  style.css           the two looks: colours, fonts, sizes (CSS variables)
  play.css            layout and animation of every screen
  art.js  audio.js  signin.js   small helpers that need the browser
  src/
    main.rs           starts the app
    pages/            whole screens (sign in, profiles, hero, play, parent...)
    components/       smaller parts (keypad, pictures, lesson, contents...)
    art.rs  api.rs    loading drawings; talking to the server
```

## 1. The browser gets HTML, CSS and a program

A web page is three layers.

- **HTML** says *what is on the page*: a heading, a paragraph, a button.
- **CSS** says *how it looks*: colour, size, where it sits.
- **A program** (here, our Rust compiled to *WebAssembly*, a format browsers
  can run) says *what happens when you tap something*.

`index.html` is nearly empty (`<body></body>`). When it loads, the Rust
program starts (`src/main.rs`) and **builds the page's HTML from Rust**.
That is the Leptos library at work.

## 2. Components and `view!`

A **component** is a Rust function that returns a piece of page. The
`view!` macro lets you write that piece as HTML-like tags inside Rust.
Here is a real one, `src/components/keypad.rs`, cut down:

```rust
#[component]                       // "this function is a component"
pub fn Keypad(on_key: Callback<Key>) -> impl IntoView {
    view! {                        // what comes next is HTML-like
        <div class="keypad">       // a box, with the CSS class "keypad"
            <button on:click=move |_| on_key.run(Key::Back)>
                "⌫"                // text goes in quotes
            </button>
        </div>
    }
}
```

Things to notice:

- `class="keypad"` is the **bridge to CSS**: a rule written `.keypad { ... }`
  in `style.css` or `play.css` styles this box. Most of what you will want
  to change is a class name plus its rule.
- `{ ... }` inside `view!` puts a Rust value in the page: `{label}`.
- `on:click=...` says what to do when it is tapped.
- A component is used like a tag: `<Keypad on_key=on_key />` (see
  `pages/play.rs`). The things after the name (`on_key=`) are its
  *properties*, the function's arguments.
- Components can contain other components. `pages/root.rs` chooses a page,
  `pages/play.rs` is the book, and it uses `components/lesson.rs`,
  `keypad.rs`, `pictures.rs` and so on.

## 3. Signals: why the page changes

A **signal** is a value the page *watches*. When it changes, only the parts
of the page that read it are redrawn. No "redraw everything" step.

```rust
let step = RwSignal::new(0);       // a signal holding 0
step.get()                         // read it
step.set(3)                        // change it
step.update(|s| *s += 1)           // change it, based on its old value
```

Inside `view!`, a `{move || ...}` block re-runs whenever a signal it reads
changes. `components/lesson.rs` does this: the picture and the words are
`{move || ...}` blocks that read `step`, so tapping "Next" (which does
`step.update(...)`) redraws them.

The biggest signal is `screen` in `pages/play.rs`: an `enum` saying which
screen is showing (cover, a lesson, a problem, Ada's hint, the contents...).
Changing it changes the screen. `pages/root.rs` does the same with `route`
for the pages outside the book (sign in, profiles, grown-ups).

Two more words you will see:

- **`Callback`**: a function a child component can call to tell its parent
  something ("the key 7 was tapped").
- **`Effect`**: code that re-runs when a signal changes (used to play Ada's
  voice when a beat changes).

## 4. CSS: where the looks live

Two files:

- **`style.css`** is the *themes*: colours, fonts and sizes, written as
  **CSS variables**, which are named values starting with `--`.
- **`play.css`** is the *layout* of screens and the *animations*. It uses the
  variables but never writes a colour itself.

The palette (`docs/PALETTE.md`) is seven colours with fixed names. In
`style.css` each look gives them values:

```css
[data-look="gentle"]      { --ink: #3b2a1e;  --warm: #b07a4a; ... }
[data-look="exceptional"] { --ink: #1e1a16;  --warm: #1f4e8c; ... }
```

and everywhere else a rule says `color: var(--ink)`. The page's outer `div`
carries `data-look="gentle"` or `"exceptional"` (set in `pages/play.rs`), and
the matching block applies. **Switching look is only that one attribute
changing**; every colour follows. This is also why *no colour may be typed in
any other rule*: it would not switch.

Sizes work the same way: `--text-size`, `--gap`, `--corner` (roundness) are
at the top of `style.css`. Change `--gap` and the spacing of the whole book
changes.

A CSS rule is `selector { property: value; }`. The selector picks things:
`.keypad` (everything with class "keypad"), `.keypad button` (buttons inside
it), `button:disabled` (a button that is switched off). When two rules
disagree, the more specific one wins, and between equals, the later one.

## 5. Layout: flex and grid

Two tools place things.

- **Flex** lines things up in a row (or column) and spaces them. `.says`
  (Ada beside her words) is flex:
  `display: flex; align-items: center; gap: var(--gap);`
- **Grid** makes rows *and* columns. `.keypad` is a grid of three columns:
  `display: grid; grid-template-columns: repeat(3, var(--key-size));`.
  `.choices` (the big picture buttons) uses
  `repeat(auto-fit, minmax(7rem, 1fr))`: "as many columns as fit, each at
  least 7rem wide". That is how the same page works on a phone and a laptop
  without a separate design for each.

The page itself is a column: `.book` in `style.css` has a
`max-width` (44rem) and `margin: 0 auto` (centred), so on a laptop it is a
page in the middle, and on a phone it fills the width.

**Units.** `rem` is "the size of the text on the page" (so 2rem is twice the
text). Use `rem` for sizes so everything scales with the reader's text size.
`%` is a share of the parent. `px` is a screen dot (use only for lines).

**Phone vs laptop.** A `@media (min-width: 48rem) { ... }` block applies only
on wider screens. There is one in `style.css` (bigger text and padding).

## 6. The drawings (SVG)

Pictures are **SVG**: text files that describe shapes. You draw them in
Inkscape (`docs/ART_GUIDE.md`) and save them under `assets/`.

How one gets on the screen (`src/art.rs`):

1. The program asks the server for the file (`/assets/lessons/...`).
2. `recolour` swaps each palette hex colour in the text for
   `var(--ink)`, `var(--warm)` and so on. **So one drawing is Parchment in
   the Gentle look and Byrne's colours in the Exceptional look**: it is the
   same CSS-variable trick as section 4.
3. The text is placed into the page (`inner_html=...`), so the drawing's
   parts become part of the page and CSS can reach them.
4. CSS then shows, hides and moves parts **by their id**. You named the
   layers in Inkscape (`hatch-open`, `gear`...); `play.css` says things like
   `.machine[data-run="true"] #gear { animation: spin 1400ms ... }`.

If a file is missing, the page keeps a plain placeholder shape, so nothing
breaks while you are still drawing.

## 7. Trying a change and seeing it

**Fastest, nothing saved: the browser's inspector.**

1. Run the app (`cargo run -p primer-server`, open the address).
2. Right-click any part of the page, choose **Inspect**.
3. In the *Styles* panel you see the CSS rules that apply to that part.
   Click a value (a colour, a size) and change it: the page changes at once.
   Use the up/down arrow keys on numbers.
4. Nothing is saved. Reload and it is gone. When you like it, write the same
   change into `style.css` or `play.css`.

Phone shape: in the inspector, the *device toolbar* button (a phone and
tablet icon) lets you pick a phone size.

**To save a change.** Edit `style.css` or `play.css`, then rebuild the web
part, `cd crates/web && trunk build`, and reload. (The first build is slow;
later ones are quicker.) CSS is copied unchanged into `crates/web/dist/`, and
the server serves that folder, so for a *CSS-only* experiment you can also
edit `crates/web/dist/style.css` and just reload, with no build. Copy what
you keep back into `crates/web/style.css`, because the next `trunk build`
overwrites `dist/`.

**Rust changes** (anything in `src/`) need `trunk build`. Wording of the
book is not in this folder at all: that is `content/*.toml`.

## Where to start looking, by what you want to change

| You want to change | Look in |
|---|---|
| a colour, the font, how round corners are, text size | `style.css`, top (the variables) |
| spacing or placement on one screen | `play.css`, the class of that screen (find it in `src/pages/*.rs` or `src/components/*.rs`) |
| what a button says | the `view!` in `src/pages/` or `src/components/` |
| the look of a button | `.next`, `.go`, `.dont-know`, `.link`, `.choice` in `play.css` |
| a picture | `assets/` (draw it) and `crates/web/src/components/pictures.rs` (how it is placed) |
| what a page does when tapped | the `on:click` next to that button |

## How batch 1 was done

Each fix from `docs/ui-audit/README.md` (items 1 to 9), and the rule or
component that did it. Read the diff next to this list.

1. **Answer underline too low.** `.equation .box` in `style.css` got
   `line-height: 1.1`. The page's line height is 1.6, so a bordered box inherited
   a tall line and its bottom border fell far below the digits.
2. **Privacy list hard to read.** `.panel ul` in `play.css`: `text-align: left`
   (the panel is centred), a `max-width`, left padding, and `.panel li` spacing.
3. **Blue link, odd form fields.** `.book a { color: var(--ink-soft) }` (browsers
   colour links blue unless a rule says otherwise); `.panel label` spaced so the
   two pickers wrap tidily.
4. **Hero picker.** In `pages/hero.rs` the "type your own" block now comes before
   the names (order in `view!` is order on the page); `.choices.names` and
   `.names .choice` make the buttons shorter, and `white-space: nowrap` stops
   "al-Khwarizmi" breaking at its hyphen.
5. **Empty states.** `.error` lost `min-height` (an empty message no longer
   leaves a gap); the parent page shows "No readers yet" once the list has
   loaded (`None` until it arrives, so it never flashes); each reader button in
   `pages/profiles.rs` shows age and look in a `.small` line.
6. **Keypad.** `.keypad` is now `repeat(3, 1fr)` with a `max-width` (columns share
   the width instead of being fixed pixels), `--key-size` grew to 4rem, and
   "I don't know" moved *inside* the grid in `components/keypad.rs` so it fills
   the empty last slot (`.keypad .dont-know` for its smaller words).
7. **Machine numbers.** `.machine-numbers .machine-io`: ink colour, centred, a thin
   rule under each line, like a ruled caption.
8. **Slab behind the single machine.** `.machine svg > rect:first-of-type` is now
   hidden for one machine too (it already was for the joined view).
9. **Card hugging the top on a laptop.** In the `min-width: 48rem` block of
   `style.css`, `.shell` gets `padding-top: clamp(1rem, 8vh, 4rem)`: some air,
   that grows with the window but stays small. (True vertical centring would
   make the page jump between beats of different height, so it is not used.)
