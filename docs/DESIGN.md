# Primer v1 — Design

An adaptive arithmetic tutor for kids aged 5–8, shaped like an enchanted book (the
Primer from `PROMPT.md`). Mostly used on phones, also Chromebooks. One child, one
page, one problem at a time. It notices *how* the child gets things wrong and
turns to the right next page. Hosted at `primer.burvy.dev` on the home server.

## Principles

1. **Pure core, I/O at the edges.** Every decision is plain data plus pure
   functions: `(state, event) -> new state`. Browser, database, clock, randomness
   and AI calls live outside the core and hand it data.
2. **Code does the maths; AI does judgment.** TypeSafe's docs say Jev "is not a
   calculator" ([jev-1.13 jagged edges](https://docs.typesafe.ai/model-jaggedness/jev-1.13)).
   Every sum, right/wrong check and difficulty step is plain Rust.
3. **A human writes every word a child reads** (v1). AI judges; it doesn't write.
4. **Simple first.** Few crates, one server, one database file, no npm.

## Stack: Leptos CSR + Trunk + axum (like burvy-dev)

- **Known already:** same build, deploy, Cloudflare and Google sign-in as `plgroup`.
- **A book, not a form:** page turns, keypad, sounds and counters need client
  code; with Leptos it is Rust. Bevy wasm games embed later as in burvy-dev.
- **FP stays clean:** the pure `core` crate builds for native *and* wasm, is
  tested with plain `cargo test`, and knows nothing about Leptos or axum.
- One origin: axum serves the Trunk `dist/` *and* `/api/*`, so no CORS.

```
crates/core     PURE: problem, skill ladder, generate, mistake, learner, policy, content checks
crates/api      request/response structs shared by server and web (serde only)
crates/server   axum: routes, auth, db (SQLite), jev client, content loader
crates/web      Leptos CSR: book layout, two looks, keypad, manipulatives, sound
crates/lint     authoring tool: `primer-lint` checks content files (code + Jev)
content/        the human-written book: chapters as TOML files
assets/         art, fonts, sounds (placeholders until the human supplies them)
```

## Core data (in words)

- `Problem { a, op, b }`; `Op = Add | Sub` (v1). Invalid problems can't be built.
- `Skill`: one chapter, e.g. "make ten", "3 + □ = 7". The ladder is data: each
  skill lists the skills it `requires` and its `track` (only `Core` in v1), so it
  can branch into tracks later without code changes.
- `Page = Lesson(step) | Practice(Problem) | Story` — what the child sees next.
- `Stage = Pictorial | NumberLine | Abstract` — how the problem is *shown*.
- `Look = Gentle | Exceptional` — the child's chosen look and track (below).
- `Attempt { problem, stage, look, answer, outcome, ms_taken, strategy }` — one event.
- `Outcome = Correct | Wrong(MistakeKind) | DontKnow`; `MistakeKind = OffByOne |
  WrongOperation | DigitsSwapped | CarryOrBorrow | Guess | Unknown`.
- `Learner` = lessons seen + mastery per (skill, stage). **A fold over events:**
  `events.iter().fold(Learner::default(), Learner::apply)`. The database stores
  events; the core derives state; no "progress" column to drift.
- `policy::next(&Learner, Look, seed) -> Page` picks skill, stage, problem, text.

## How it teaches (and why)

Each skill moves **concrete → pictorial → abstract** (CPA, from Bruner; the
backbone of Singapore maths, [Leong, Ho & Cheng 2015](https://repository.nie.edu.sg/server/api/core/bitstreams/700a4f49-3cdd-4365-9802-92b8634b39eb/content)):
ten frames and counters, a number line, then the bare equation
([NCTM 2014](https://nctm.confex.com/nctm/2014AM/webprogram/Handout/Session22473/NCTM%202014%20handout.pdf)).
Linear number games improve young children's number sense ([Ramani, Siegler & Hitti 2012](https://education.umd.edu/sites/default/files/uploads/Ramani%20et%20al.%2C%202012%20%281%29.pdf);
[WWC](https://ies.ed.gov/ncee/wwc/study/73664)), so the number line is its own stage
([EEF review, ages 3–7](https://pure.ulster.ac.uk/ws/files/88211840/Early_maths_evidence_review_ForPUBLICATION_2020_11_20.pdf)).
Real objects (concrete) are the grown-up's part; `PLAYTEST.md` suggests some.

**Teach, then practise.** Every skill opens with a short **lesson**: Ada works one
example through the stages (counters → number line → equation), the child taps
along. Then come problem sets.

**Adaptivity (plain code):** 3 right in a row → next stage; abstract mastered →
next skill; 2 wrong → back one stage plus the hint for that mistake kind; two
step-backs → offer the lesson again; slow answers, "I don't know" taps or a
frustration signal → easier page, kind words.

**Feel smart, never babyish.** Real notation in a maths font: `3 + 4 = □`
(the box is an unknown). **Missing-number problems** (`3 + □ = 7`) are skills of
their own: early algebra, and most of the Exceptional look's extra work. Real words
with a gloss: *sum*, *difference*, *equation*. Skills are chapters; a mastered
skill becomes a signed "theorem" page. No baby talk, no cartoon mascot; the
narrator speaks to the child as a capable young reader.

## The story and the two looks

**Story:** a fairy-tale journey mirroring the Primer: a young hero, companions,
castles and a logic castle whose doors open by reasoning (in the spirit of Castle
Turing). Names and places are our own, not Stephenson's.

**The hero is the child,** as Nell is in the Primer. The child names the hero:
pick from a list of fairy-tale names, or type one. A typed name must pass a code
rule (one word, letters only, ≤12) and a Jev Noul ("does this look like a real
person's name or other identifying detail?"); if it fails, the list is offered.

**The narrator is Ada** (a nod to Ada Lovelace), a *ractor*-style voice like
Miranda in *The Diamond Age*: the human author behind the book.

**Two looks, one app.** A look is a CSS theme (`data-look` on `<html>`, CSS
variables) **plus** a `Look` value the policy reads. Switch any time; progress is
shared. The human named them **Gentle** and **Exceptional**.
- *Gentle* (storybook style): warm illustrated pages, ink and watercolour, CPA at normal pace.
- *Exceptional* (treatise style): a Victorian mathematical treatise in the manner of Oliver Byrne's
  1847 coloured *Euclid*: cream paper, red/yellow/blue shapes, serif type,
  "Proposition" headings. Harder track: fewer pictorial pages, missing-number
  forms (`□ + 4 = 7`), column form sooner, and "How did you know?" questions.
  Chapters finished here earn a "Proved Exceptional" seal worth bragging about.

## Request flow: child answers

1. `GET /api/child/{id}/next` → page (problem, stage, text ids). Child taps the
   keypad (digits, ⌫, "I don't know"), then `POST .../answer { page_id, answer, ms_taken, explanation? }`.
2. **Pure:** check; if wrong, `mistake::detect`. **Edge, only if needed:** one Jev
   call (below); right answers with no explanation call no AI.
3. Store the `Attempt`, fold it into the `Learner`, `policy::next`, reply; page turns.

## Jev at runtime: the adaptive AI

One `POST https://api.typesafe.ai/v1/systemone` (`Authorization: Bearer`, body
`{ model: "jev-latest", state, questions }`, [API](https://docs.typesafe.ai/api),
[primitives](https://docs.typesafe.ai/primitives)); serde + reqwest, no SDK; all
questions in one request ([fan-out](https://docs.typesafe.ai/patterns/fan-out)).
- **Mistake kind** when code finds no signature: Choice over `MistakeKind`, with
  code-computed facts in `state` (difference, digits), so Jev does no maths.
- **Reading the child's explanation** ("How did you know?", typed, Exceptional look
  or older kids): Choice over strategies (counted all, counted on, made ten,
  doubles, just knew, unclear). The policy uses it: "counted all" at age 7 →
  more make-ten pages. Gentle-look kids tap a picture card instead; no AI.
- **Frustration:** Noul on the explanation text ("does this child sound upset or
  stuck?"), combined in code with timing and "I don't know" taps.
- Low `confidence` → `Unknown` ([confidence routing](https://docs.typesafe.ai/patterns/confidence-routing)).
  The safe option goes first, since Jev leans toward the first Choice option.
- Jev down or no key → `Unknown`; the tutor still works (a fake proves it).

## Content: written by a human, checked by machine

All text lives in `content/` as **TOML**, one file per chapter
(`content/03-making-ten.toml`): plain text, readable diffs, parsed by `serde`.
Entries: id, skill, stage, look (`both` default), kind (`lesson`, `word_problem`,
`hint`, `praise`, `story`), text. A `lesson` is ordered steps (stage, Ada's line,
worked example). Word problems use `{a}`/`{b}` slots that code fills.
`docs/WRITERS_GUIDE.md` says what each chapter needs (a lesson, a few word
problems per stage, a hint per mistake kind), reading level, tone, word limits.

**`primer-lint`** gives the writer feedback:
- *Code checks (pure, in `core`):* file parses, ids unique, slots valid, no stray
  digits, word limit, every (skill, stage, mistake kind) has a hint (coverage).
- *Jev lint:* reading level (Score, ages 5/6/7/8+), gives the answer away (Noul),
  kid-safe (Noul), operation matches skill (Choice); cached by text hash.
- The server runs the code checks at start and refuses broken content.

*Later:* `primer-lint draft` (a strong model drafts, the human rewrites);
generated text for children may return at scale, gated by the same lint.

**Keys stay server-side.** `jev-key.env` (gitignored; `PRIMER_KEY_DIR` on the
server) holds only the raw key, read and trimmed into `secrecy::SecretString`.
Only `jev.rs` uses it; the browser talks only to `/api`.

## Privacy: plan for COPPA now

COPPA is the US Children's Online Privacy Protection Act.
- **Parent owns everything.** Google sign-in as in `plgroup` (`tokeninfo`, random
  session token, `Bearer`). Children have no login, email or password.
- **Child profile:** hero name (from the list, or checked as above), age band,
  look. No real names anywhere. Jev sees a typed hero name once, to check it.
- **Minimal data:** attempts (problem, answer, outcome, time, strategy label).
  Explanations go to Jev, then are **dropped**; only the label is kept.
- **Delete on request:** delete a child (attempts cascade) or the whole account;
  hard delete. JSON export too. Leaving child mode needs a grown-up check.
- **SQLite** (sqlx, static SQL): `parents`, `sessions`, `children`, `attempts`
  (the event log); content is files. DB in `C:\burvy\data\primer\`, backed up.
- Fonts and assets self-hosted: no third-party calls from a child's device.
- A privacy page lists what's kept and the one processor (TypeSafe). The human
  should confirm TypeSafe's data terms and the current FTC rule (not legal advice).

## Art and assets

Placeholders now, a row per file in `assets/MANIFEST.md` (name, size, format,
look, who makes it, status). CSS variables hold colours, fonts and textures, so
real art is a drop-in file swap.

| Asset | Placeholder | Human could make or pick |
|---|---|---|
| Paper textures: Gentle (warm), Exceptional (cream) | flat colour + CSS shading | scan or photograph paper |
| Ada, the narrator: portrait, small vignette | silhouette SVG | draw her look |
| The hero (name-neutral), companions, castles, logic castle | grey outline SVG | draw the cast and places |
| Byrne-style figures, drop caps, ornaments, seals | CSS/SVG, plain letters | draw, refine palette |
| Manipulatives: ten frame, counters, number line | drawn in code as SVG | style per look |
| Fonts: Gentle serif, Exceptional serif, maths | system serif | pick, e.g. Andika (early readers), STIX Two Math |
| Sounds; later Ada's recorded voice | silence | record or pick CC0; mute always |

## Later (not v1)

**Games:** one single-player bevy wasm mini-game as a chapter reward, embedded
like burvy-dev's `floret`/`game`. No multiplayer: children must not meet strangers
online, and networking adds chat and moderation that COPPA makes costly.
**Tracks ("majors"):** after the core ladder the child may pick a track, each
with a short "what's in it for you" page: algebra-first, then pure maths, applied
maths, maybe programming and functional thinking. The ladder's `requires` and
`track` fields already allow this; v1 ships only `Core`.

Also later: generated text at scale, reading and writing, multiplication,
a parent progress view, voice.

## Crates

`axum`, `tokio`, `tower-http` (server, static files, tracing) · `sqlx` sqlite
(static queries) · `reqwest` rustls (Jev, Google `tokeninfo`) · `serde`,
`serde_json`, `toml` (JSON and content files) · `secrecy` (unprintable keys) ·
`rand` (seeded RNG into the pure generator) · `thiserror`, `tracing` (edge errors;
logs never hold keys or nicknames) · `clap` (lint CLI) · `leptos` csr,
`leptos_router`, `gloo-net`, `web-sys` (web app, as in burvy-dev).
