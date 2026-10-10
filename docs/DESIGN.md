# Primer v1 — Design

An adaptive arithmetic tutor for kids aged 5–8, shaped like an enchanted book (the
Primer from `PROMPT.md`). Mostly used on phones, also Chromebooks. One child, one
page, one problem at a time. It notices *how* the child gets things wrong and
turns to the right next page. Hosted at `primer.burvy.dev` on the home server.

## Principles

1. **Pure core, I/O at the edges.** Decisions are data plus pure functions,
   `(state, event) -> new state`; browser, database, randomness and AI are edges.
2. **Code does the maths; AI does judgment.** TypeSafe's docs say Jev "is not a
   calculator" ([jev-1.13 jagged edges](https://docs.typesafe.ai/model-jaggedness/jev-1.13)).
   Every sum, right/wrong check and difficulty step is plain Rust.
3. **A human writes every word a child reads** (v1). AI judges; it doesn't write.
4. **Simple first.** Few crates, one server, one database file, no npm.

## Stack: Leptos CSR + Trunk + axum (like burvy-dev)

The human already knows it (build, deploy, Cloudflare, Google sign-in); a book
needs client code (page turns, keypad, sounds), and with Leptos that is Rust; bevy
games embed later. The pure `core` crate builds for native and wasm and knows
nothing of Leptos or axum. axum serves `dist/` and `/api/*` from one origin.

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
- `Skill`: one chapter, e.g. "make ten" (`3 + □ = 10`). The ladder is data: each
  skill lists what it `requires`, its `track` (only `Core` in v1), its `words`
  (vocabulary ids) and its `patterns` (below), so it can branch into tracks later.
- `ProblemSet { id, skill, problems, pattern?, notice? }` — an ordered run of
  problems; written by a human, or built from a `Pattern` by code.
- `Pattern` — a named shape for a run (successor, swap, doubles, bonds…): data.
- `Page = Lesson(step) | Practice(Problem) | Story` — what the child sees next.
- `Stage = Pictorial | NumberLine | Abstract` — how the problem is *shown*.
- `Look = Gentle | Exceptional` — the child's chosen look and track (below).
- `Attempt { set_id, problem, stage, look, answer, outcome, ms_taken, strategy }`.
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
example through the stages, introducing the chapter's words. Then problem sets.
**Adaptivity (plain code):** 3 right in a row → next stage; abstract mastered →
next skill; 2 wrong → back one stage plus the hint for that mistake kind; two
step-backs → offer the lesson again; slow answers, "I don't know" taps or a
frustration signal → easier page, kind words.

**Feel smart, never babyish.** Real notation in a maths font: `3 + 4 = □`
(the box is an unknown). **Missing-number problems** (`3 + □ = 7`) are early
algebra and most of the Exceptional look's extra work. A mastered skill becomes a
signed "theorem" page. No baby talk, no mascot; Ada speaks to a capable reader.

## The chapters: words and patterns

Each chapter brings a few **real words**, introduced by Ada in its lesson with a
plain gloss, then collected in the child's "My words" page. Gentle shows the
gloss first; Exceptional uses the formal word first. Crossing ten (8 + 5) waits
for chapter 8.

| # | Chapter | Example | Words (Gentle gloss → formal) | Patterns |
|---|---|---|---|---|
| 1 | Count on within 5 | 3 + 1 | the next number → *successor*; +1 machine → *function* | successor |
| 2 | Add within 10 | 3 + 4 | *sum*, *addend*, *equation*, *equals* ("the same as"); swap rule → *commutative*; *doubles* | successor, swap, doubles |
| 3 | Missing addend | 3 + □ = 7 | *unknown*; fact family | family |
| 4 | Make ten (bonds) | 3 + □ = 10 | *number bond*; one up, one down → *compensation* | bond walk |
| 5 | Subtract within 10 | 7 − 3 | *difference*; the number before → *predecessor*; undo → *inverse* | predecessor, undo |
| 6 | Add within 20, no carry | 13 + 4 | *ones*, *tens* → *place value* | parallel (3+4, 13+4) |
| 7 | Subtract within 20, no borrow | 17 − 3 | *pattern* | parallel |
| 8 | Add within 20, crossing ten | 8 + 5 | bridge through ten; *near doubles* | bridge, near doubles |
| 9 | Subtract within 20, crossing ten | 13 − 5 | bridge back through ten | bridge back |

A skill's `words` field lists word ids; the text (gloss, formal word, Ada's line)
is content in `content/words.toml`. Lint checks every id exists, taught once.

## Problem sets: written first, then patterned

**Written first.** Each chapter opens with hand-written, ordered problem sets in
its content file (drafted by Cline, edited by the human/ractor), for example the
swap run `3 + 5`, `5 + 3`, `2 + 6`, `6 + 2` with a `notice` line ("Did you see?
Swapping the addends kept the sum. That is the *commutative* rule."). The policy
serves them in order.

**Then patterned, never plain random.** When a child has used up the written sets
(and would start seeing repeats), code builds new sets by walking a `Pattern`:

- *successor*: `3+4, 4+4, 5+4` (the sum goes up by one); *predecessor* likewise.
- *swap*: `a+b` then `b+a`. *doubles*: `2+2, 3+3, 4+4`. *near doubles*: `6+6` then `6+7`.
- *family*: `3+4=□, 3+□=7, □+4=7, 7−4=□`. *undo*: `3+4` then `7−4`.
- *bond walk*: `1+□=10, 2+□=10, 3+□=10…`. *parallel*: `3+4` then `13+4`.
- *bridge*: `8+2`, then `8+5` ("make ten first"); *bridge back*: `13−3`, then `13−5`.

`pattern::walk(pattern, start, len) -> Vec<Problem>` is pure; randomness only
picks the pattern (from the skill's list) and the starting numbers, and every
problem must still pass `skill.allows`. A generated set ends with the pattern's
`notice` line (human-written, one per pattern), naming the idea just used.

## The story and the two looks

**Story:** a fairy-tale journey mirroring the Primer: a young hero, companions,
castles and a logic castle whose doors open by reasoning (in the spirit of Castle
Turing). Names and places are our own, not Stephenson's. **The hero is the
child:** pick a fairy-tale name from a list or type one; a typed name must pass a
code rule (one word, letters only, ≤12) and a Jev Noul ("a real person's name or
identifying detail?"), else the list returns. **The narrator is Ada** (for Ada
Lovelace), a *ractor*-style voice like Miranda in *The Diamond Age*.

**Two looks, one app:** a CSS theme (`data-look` + CSS variables) plus a `Look`
value the policy reads. Switch any time; progress is shared.
- *Gentle* (storybook): warm illustrated pages, ink and watercolour, normal pace.
- *Exceptional* (treatise, after Oliver Byrne's 1847 coloured *Euclid*): cream
  paper, red/yellow/blue shapes, "Proposition" headings, formal words first.
  Harder: fewer pictorial pages, more missing-number forms, "How did you know?"
  questions, and a "Proved Exceptional" seal worth bragging about.

## Request flow: child answers

1. `GET /api/child/{id}/next` → page. Child taps the keypad (digits, ⌫, "I don't
   know"), then `POST .../answer { page_id, answer, ms_taken, explanation? }`.
2. **Pure:** check; if wrong, `mistake::detect`. **Edge, only if needed:** one Jev
   call; right answers with no explanation call no AI.
3. Store the `Attempt`, fold it into the `Learner`, `policy::next`, reply.

## Jev at runtime: the adaptive AI

One `POST https://api.typesafe.ai/v1/systemone` (`Authorization: Bearer`, body
`{ model: "jev-latest", state, questions }`, [API](https://docs.typesafe.ai/api),
[primitives](https://docs.typesafe.ai/primitives)); serde + reqwest, no SDK; all
questions in one request ([fan-out](https://docs.typesafe.ai/patterns/fan-out)).
- **Mistake kind** when code finds no signature: Choice over `MistakeKind`, with
  code-computed facts in `state`, so Jev does no maths.
- **Strategy** from the typed "How did you know?" (Exceptional or older kids):
  Choice (counted all, counted on, made ten, doubles, just knew, unclear); e.g.
  "counted all" at age 7 → more bond sets. Gentle kids tap a picture card; no AI.
- **Frustration:** Noul on the explanation, combined in code with timing and
  "I don't know" taps.
- Low `confidence` → `Unknown` ([confidence routing](https://docs.typesafe.ai/patterns/confidence-routing));
  safe option first (Jev leans to the first Choice). Jev down → `Unknown`.

## Content: written by a human, checked by machine

All text lives in `content/` as **TOML**, one file per chapter
(`content/04-make-ten.toml`), parsed by `serde`. Entry kinds: `lesson` (ordered
steps: stage, Ada's line, worked example), `problem_set` (ordered problems such
as `"3 + 5"` or `"3 + □ = 10"`, optional pattern tag and `notice`),
`word_problem` (`{a}`/`{b}` slots), `hint`, `praise`, `story`. Each has id, skill,
stage and look (`both` default). `docs/WRITERS_GUIDE.md` says what a chapter
needs, reading level, tone and word limits.

**`primer-lint`:** *code checks* (parses, ids unique, slots valid, no stray digits,
word limits, problems keep the skill's promise, coverage of lessons, hints and
words) and *Jev lint* (reading level Score, answer given away Noul, kid-safe Noul,
operation Choice; cached by text hash). The server refuses broken content.
*Later:* `primer-lint draft` (a strong model drafts, the human rewrites);
generated text for children may return at scale, gated by the same lint.

**Keys stay server-side.** `jev-key.env` (gitignored; `PRIMER_KEY_DIR` on the
server) holds only the raw key, trimmed into `secrecy::SecretString`. Only
`jev.rs` uses it; the browser talks only to `/api`.

## Privacy: plan for COPPA now

COPPA is the US Children's Online Privacy Protection Act.
- **Parent owns everything:** Google sign-in as in `plgroup`; children have no
  login. Child profile = hero name, age band, look. No real names anywhere.
- **Minimal data:** attempts only. Explanations go to Jev, then are **dropped**;
  only the strategy label is kept.
- **Delete on request** (child or account, hard delete, attempts cascade), JSON
  export, grown-up check to leave child mode, self-hosted fonts and assets.
- **SQLite** (sqlx, static SQL): `parents`, `sessions`, `children`, `attempts`;
  DB in `C:\burvy\data\primer\`, backed up. Content is files, not rows.
- A privacy page names the one processor (TypeSafe). The human should confirm
  TypeSafe's terms and the current FTC rule (not legal advice).

## Art and assets

Placeholders now; one row per file in `assets/MANIFEST.md`; CSS variables make
real art a drop-in swap. The human could make or pick: paper textures (warm,
cream); Ada's portrait; the hero (name-neutral), companions, castles, logic
castle; Byrne-style figures, drop caps, ornaments, seals; manipulative styling
(ten frame, counters, number line are SVG in code); fonts (e.g. Andika for early
readers, STIX Two Math); sounds (CC0 or recorded, always mutable); later Ada's voice.

## Later (not v1)

**Games:** one single-player bevy wasm reward game, embedded like burvy-dev's
`floret`; no multiplayer (no strangers for kids; chat and moderation are costly
under COPPA). **Tracks ("majors"):** after the core ladder, optional tracks with a
"what's in it for you" page: algebra-first, pure, applied, maybe programming and
functional thinking; `requires` and `track` already allow it. Also: generated
text at scale, reading and writing, multiplication, parent progress view, voice.

## Crates

`axum`, `tokio`, `tower-http` · `sqlx` sqlite · `reqwest` rustls · `serde`,
`serde_json`, `toml` · `secrecy` · `rand` (seeded, injected) · `thiserror`,
`tracing` (never logs keys or names) · `clap` (lint CLI) · `leptos` csr,
`leptos_router`, `gloo-net`, `web-sys` (as in burvy-dev).
