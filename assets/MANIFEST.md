# Asset manifest (PLACEHOLDERS unless said otherwise).
#
# How this works for the human (the Inkscape workflow): every colour, font,
# texture path and spacing the code uses is a CSS variable in
# `crates/web/style.css` (`--paper`, `--texture-ada`, ...). To drop in real
# art, draw the file and keep the same path — or edit the one variable line.
# No Rust or CSS structure needs to change.
#
# Drawn by code (no files here): the tower, number line, ten frame and
# counters are SVG the app draws at runtime (sizes must fit the child's
# numbers). Hand-drawn highlights are layers inside the SVGs below; code only
# shows, hides or animates them by id (for example `#hatch-open`).

| path | what it is | size | layers | look(s) | status |
|---|---|---|---|---|---|
| `assets/fonts/Andika-Regular.ttf` | Reading text font (SIL OFL) | 670 KB | n/a | both | real font, self-hosted |
| `assets/fonts/STIXTwoMath-Regular.ttf` | Equation font (SIL OFL) | 1518 KB | n/a | both | real font, self-hosted |
| `assets/fonts/OFL-Andika.txt` | Andika licence text | 5 KB | n/a | both | licence |
| `assets/fonts/OFL-STIXTwoMath.txt` | STIX Two Math licence text | 5 KB | n/a | both | licence |
| `assets/textures/paper-gentle.svg` | Warm paper grain (Gentle) | 1 KB | none | gentle | placeholder |
| `assets/textures/paper-exceptional.svg` | Cream paper with faint grid (Exceptional) | 1 KB | none | exceptional | placeholder |
| `assets/ada/ada-silhouette.svg` | Ada narrator portrait | 1 KB | none | both | placeholder |
| `assets/ornaments/divider.svg` | Section divider ornament | 1 KB | none | both | placeholder |
| `assets/seals/proved-exceptional.svg` | "Proved Exceptional" seal, Byrne red/yellow/blue | 1 KB | none | exceptional | placeholder |
| `assets/lessons/function-machine.svg` | Chapter 1 +1 machine | 3 KB | body, funnel, chute, front-panel, input-glow, output-glow, hatch-closed, hatch-open, block, gear, pipe | both | placeholder |
| `assets/textures/deckle.svg` | Torn page edge of the Gentle look | 1 KB | path | gentle | placeholder |
| `assets/lessons/marks.svg` | Tick and cross beside a checked answer | 1 KB | tick, cross | both | placeholder |
| `assets/lessons/highlights.svg` | Hand-drawn highlight strokes | 1 KB | circle, underline, arrow, bracket | both | placeholder |
