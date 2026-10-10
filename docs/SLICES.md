# Primer v1 — Slice plan

Each slice is one reviewable sitting (about 100–200 lines of diff), passes
`cargo test` and `cargo clippy -- -D warnings`, and leaves `main` building.
Pure core first, edges last. Names refer to `docs/DESIGN.md`.

Owners: `cline` = well-specified, self-contained work (5 of 15);
`kevin` = most code (7); `stanley` = the tricky, design-heavy slices (3).

---

## 1. Workspace and problems — `kevin`
- **Goal:** workspace with `core`, `api`, `server`, `web`, `lint` (shells that
  build; `web` has a Trunk `index.html`). In `core`: `Op`, `Problem`, `answer()`,
  `Display` ("3 + 4"); subtraction can't go below zero.
- **Files:** `Cargo.toml`, `crates/*/Cargo.toml`, `crates/core/src/{lib,problem}.rs`,
  `crates/web/{index.html,Trunk.toml,src/main.rs}`, `crates/{server,lint}/src/main.rs`.
- **Teaches:** workspaces; enums and `match`; invalid states unrepresentable
  (private fields + `fn new(..) -> Option<Problem>`).
- **Done when:** tests cover add, sub, `Display`, `new` rejecting `3 - 5`;
  `trunk build` succeeds.

## 2. Skill ladder as data — `cline`
- **Goal:** `const SKILLS: &[Skill]`: ~9 chapters (count on within 5, add within
  10, missing addend `3 + □ = 7`, make ten, subtract within 10, add within 20 no
  carry, …) with op, number ranges, chapter title, stages used, `requires`
  (skill ids), `track` (`Core` only), and per-`Look` tweaks (Exceptional: skip
  pictorial after 1 right, more missing-number forms).
- **Files:** `crates/core/src/{skill,look}.rs`, `lib.rs`.
- **Teaches:** const data, slices `&[T]`, iterators (`find`, `position`).
- **Done when:** tests: unique ids, non-empty ranges, every skill ends at
  `Abstract`, every `requires` id exists and the graph has no cycles.

## 3. Generation and mistake signatures — `kevin`
- **Goal:** `generate(skill, look, &mut impl Rng) -> Problem` (in range; Exceptional
  may produce `□ + b = c`) and `mistake::detect(problem, given) -> Option<MistakeKind>`.
- **Files:** `crates/core/src/{generate,mistake}.rs`, `Cargo.toml` (`rand`).
- **Teaches:** generics and trait bounds; injecting the RNG keeps it testable;
  `Option` combinators; `match` on tuples.
- **Done when:** 1,000 seeded draws per skill stay in range; same seed, same
  problem; slice 4's table passes.

## 4. Mistake and keypad test tables — `cline`
- **Goal:** ~40 `(problem, answer, expected MistakeKind or None)` cases plus keypad
  edge cases (empty, leading zero, "I don't know"). Data in a test module.
- **Files:** `crates/core/tests/mistakes.rs`.
- **Teaches:** table-driven tests.
- **Done when:** human reviews cases for kid realism. May land before slice 3
  with the test `#[ignore]`d.

## 5. Learner as a fold, and the policy — `stanley`
- **Goal:** `Event` (`LessonSeen`, `Attempt`), `Outcome`, `Strategy`, `Page`
  (`Lesson`/`Practice`/`Story`), `Learner::apply(self, &Event)`, `Learner::from_events`
  (a fold), `policy::next(&Learner, Look, seed) -> Page` with the DESIGN.md rules
  (lesson first, stage up, step back + hint, lesson again after two step-backs,
  easier on frustration, strategy-driven pages, next skill only when `requires` met).
- **Files:** `crates/core/src/{learner,policy,page}.rs`.
- **Teaches:** state as a fold over events; ownership (`apply` takes `self`);
  exhaustive matching.
- **Done when:** scripted event lists produce the expected page for both looks
  (a new skill always starts with its lesson); folding twice gives equal results.

## 6. Server, SQLite and the answer API — `kevin`
- **Goal:** axum serving `dist/` and `/api`; `api` types; sqlx SQLite with
  `children`, `attempts`; `GET .../next`, `POST .../answer` for a dev child
  (`dev-local` feature, no auth yet); `Judge` trait + `FakeJudge`.
- **Files:** `crates/server/src/{main,db,routes,judge}.rs`, `crates/api/src/lib.rs`,
  `migrations/0001_init.sql`.
- **Teaches:** async handlers, extractors, `Result` → response, thin edges
  (parse → core → store → reply).
- **Done when:** `oneshot` integration test answers three problems and the
  learner advances; restart keeps progress.

## 7. Two looks: themes, placeholders, manifest — `cline`
- **Goal:** `style.css` with CSS variables per look (`[data-look=gentle]`,
  `[data-look=exceptional]`, Byrne palette for the second); page-turn CSS;
  placeholder textures, narrator silhouette, ornaments, seals; self-hosted fonts
  with OFL licences; `assets/MANIFEST.md` listing every asset for the human.
- **Files:** `assets/**`, `crates/web/style.css`, `assets/MANIFEST.md`.
- **Teaches:** nothing new in Rust; sets up the human's art workflow.
- **Done when:** flipping `data-look` restyles a sample page with no code
  change; swapping one texture file needs no code edit; manifest is complete.

## 8. Play page, keypad, manipulatives, look switch — `kevin`
- **Goal:** Leptos components `BookPage`, `LessonPage` (Ada's steps, tap to go
  on), `Equation` (`□`), `Keypad`, `TenFrame`, `NumberLine` (SVG), `LookSwitch`; strategy picture cards (Gentle) and
  "How did you know?" box (Exceptional); calls the API with `gloo-net`; mute toggle.
- **Files:** `crates/web/src/{main,api,pages/play,components/*}.rs`.
- **Teaches:** signals as the edge of state; components as functions of props;
  the keypad as a pure reducer `(Input, Key) -> Input` living in `core`.
- **Done when:** a child plays a full chapter on a phone-width screen in both
  looks; native tests cover the keypad reducer.

## 9. Parent sign-in and sessions — `kevin`
- **Goal:** Google sign-in copied from `plgroup` (`tokeninfo`, random session
  token, `Parent` extractor), `parents` + `sessions` tables, sign-out.
- **Files:** `crates/server/src/auth.rs`, `migrations/0002_parents.sql`,
  `crates/web/src/pages/sign_in.rs`.
- **Teaches:** extractors as access control; newtypes for ids and tokens.
- **Done when:** tests: no token → 401; expired token → 401; valid → parent id.

## 10. Child profiles, hero name, delete, export, grown-up gate — `kevin`
- **Goal:** create/delete child, set look and age band; the child picks a hero
  name from the list (slice 13) or types one: pure rule, then
  `Judge::is_identifying(name)` (fake until slice 11), failing → back to the list;
  delete account; JSON export; grown-up check; privacy page route.
- **Files:** `crates/server/src/children.rs`, `migrations/0003_children_fk.sql`,
  `crates/core/src/hero_name.rs`, `crates/web/src/pages/{parent,profiles,hero}.rs`.
- **Teaches:** `ON DELETE CASCADE`; validation as pure parsing:
  `HeroName::parse(&str) -> Result<HeroName, _>` (a newtype).
- **Done when:** one parent can't read another's child; deleting a child leaves
  zero `attempts` rows; "Bob Smith", digits and 13+ letters rejected by code.

## 11. Jev runtime client — `stanley`
- **Goal:** `JevJudge`: one fan-out request for mistake kind, strategy and
  frustration; a separate small call for "is this hero name identifying?" (Noul); decode typed answers; confidence threshold; 2 s timeout; any
  error → `Unknown`. Key from `jev-key.env` via `secrecy`. Explanation text is
  never stored or logged.
- **Files:** `crates/server/src/{config,jev}.rs`, `tests/fixtures/jev_*.json`.
- **Teaches:** serde for an external API (tagged enums), `thiserror` + `?`,
  traits for real vs fake, secrets out of `Debug`.
- **Done when:** fixture tests decode into the right `Judgment`; low confidence →
  `Unknown`; `format!("{config:?}")` has no key; one `#[ignore]` live test.

## 12. Content format and loader — `kevin`
- **Goal:** TOML schema for `content/*.toml` (id, skill, stage, look, kind incl.
  `lesson` with ordered steps, text, `{a}`/`{b}` slots); serde types in `core`; pure checks (ids unique, slots
  valid, no stray digits, word limits, every skill has a lesson, hint coverage); server loads `content/`
  at start and refuses broken files; `policy` picks text ids.
- **Files:** `crates/core/src/content.rs`, `crates/server/src/content.rs`.
- **Teaches:** serde with `toml`; collecting many errors (`Vec<ContentError>`)
  instead of failing on the first.
- **Done when:** tests on good and bad sample files; the coverage report lists
  missing hints by (skill, stage, mistake kind).

## 13. Writer's guide and content templates — `cline`
- **Goal:** `docs/WRITERS_GUIDE.md` (how to write a lesson: Ada walks one worked
  example through the stages; then per stage: word problems, a hint per mistake
  kind, praise; reading level by age, tone, word limits, slots, the two looks'
  voices); `content/_template.toml`; drafts of chapters 1–2 (a lesson plus a
  handful of problem sets each) for the human to edit; `content/hero_names.toml`
  (~30 fairy-tale names, mixed, none of them common real full names).
- **Files:** `docs/WRITERS_GUIDE.md`, `content/{_template,hero_names}.toml`,
  `content/01-*.toml`, `content/02-*.toml`.
- **Done when:** slice 12's checks pass on the samples; the human signs off on tone.

## 14. `primer-lint` with Jev — `stanley`
- **Goal:** CLI: runs slice 12's code checks, then one Jev fan-out per entry
  (reading level Score, gives answer away Noul, kid-safe Noul, operation Choice);
  readable report per file; results cached by text hash in `.lint-cache.json`
  (gitignored). `draft` subcommand stubbed and marked "later".
- **Files:** `crates/lint/src/main.rs`, `crates/server/src/jev.rs` (shared
  questions, or a small `jev` crate), `.gitignore`.
- **Teaches:** `clap`; reusing the pure checks from another binary; caching by
  content hash.
- **Done when:** with `FakeJudge`, a too-hard sentence and an answer-revealing
  problem are flagged; a re-run makes zero Jev calls.

## 15. README, playtest, privacy, deploy — `cline`
- **Goal:** README (run locally, where the key goes, running without it, crate
  map, how to write content); `docs/PLAYTEST.md` (grown-up checklist, real-object
  ideas for the concrete stage); `docs/PRIVACY.md` draft; `docs/DEPLOY.md`
  mirroring burvy-dev (port, Cloudflare, data path, backups).
- **Files:** `README.md`, `docs/{PLAYTEST,PRIVACY,DEPLOY}.md`.
- **Done when:** a fresh clone can follow the README to a running app.

---

**Parallel work:** 2, 4, 7, 13 and 15 (drafts) need only DESIGN.md and can start
now. 5 needs 1–3; 6 needs 5; 8 needs 6 and 7; 9 needs 6; 10 needs 9;
10 uses 13's name list; 11 needs 6; 12 needs 2; 14 needs 11 and 12.
