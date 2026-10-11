# Art guide (for drawing in Inkscape)

The code never draws a finished picture. You draw the parts; the code places,
repeats, shows, hides and animates them by **id**. This guide lists every part
for chapters 1 and 2, in the order that matters most for a demo.

## 1. Rules for every file

| Rule | Why |
|---|---|
| One object per file, in `art/` while drawing (the files move to `assets/` when merged). | Small files, easy to swap. |
| Units **px**, document scale 1 (File > Document Properties). Use the canvas size given below. | The code places parts using these numbers. |
| Each moving part is its own **layer**. Set the layer's **id** (not just its name): select the layer group, open Object > Object Properties (Ctrl+Shift+O), type the id, press Set. | The code finds parts by id. The name in the Layers panel is not enough. |
| Use only the **palette** swatches (below). Flat fills, no gradients, no blur or filters. | The code swaps the exact colours for each look; filters are slow on Chromebooks. |
| One outline thickness: **4 px** at the given canvas size. Turn off "scale stroke width" (the 4 small buttons at the top right of the Select tool). | Every picture looks like it came from the same pen. |
| Never type text for numbers or labels. Draw an empty box and give it an id ending in `-box`. | The code writes numbers (+1, +0, 7) so one drawing serves every problem. |
| Save as **Inkscape SVG**. Do not use "Optimized SVG". | Optimizing can delete the ids. |
| Highlights (circles, underlines, arrows) are **strokes**, not filled shapes: draw them with the Pencil tool (P) in one motion, in the direction they should appear. | The code animates the stroke drawing itself, start to end. Calligraphy-tool shapes cannot be animated this way. |

### Palette (make these 5 swatches, then pick your colours)

| swatch name | use |
|---|---|
| `ink` | every outline, lines, numbers |
| `paper` | page background, the inside of shapes that should look empty |
| `warm` | main fill (wood, leather, the pouch) |
| `bright` | gold, the coins, things to look at |
| `mark` | highlights, glows (one colour that stands out) |

The Exceptional look recolours these to Byrne's red, yellow, blue and black.
You pick the Gentle colours; tell me the hex values.

## 2. Page and orientation

- The page is a **portrait** column, like a book page, about 350 px wide on a
  phone and at most 700 px on a laptop. Text sits above, the picture below.
- Lesson pictures are **landscape** (wider than tall), except the tower part.
- The cover and chapter openers are **portrait**.
- Draw at the canvas size below; the code scales the whole picture. Do not draw
  tiny details: a coin is shown about 20 px wide on a phone.

## 3. What to draw, in order

| # | file | canvas (px) | layers / ids | notes |
|---|---|---|---|---|
| 1 | `function-machine.svg` | 400 × 300 | `body`, `funnel` (top, where the number goes in), `chute` (bottom, where it comes out), `front-panel` with `label-box` inside (empty, the code writes `+1`), `input-glow`, `output-glow`, `hatch-closed`, `hatch-open`, `block` (the number-block that travels through), `gear`, `pipe` (joins two machines side by side) | Keep the current placeholder's positions roughly, so the code's animation still lines up. The two glows are hand-drawn outlines in `mark`. |
| 2 | `coin.svg` | 40 × 40 | `coin` | All coins are the same and worth 1. Thick rim, one simple mark. |
| 3 | `pouch.svg` | 160 × 160 | `pouch-back` (inside, behind the coins), `pouch-front` (front leather, in front of the coins), `string` (drawstring) | Open pouch, mouth facing up. The code puts coins between back and front, so they look like they sit inside. |
| 4 | `stone-block.svg` | 80 × 40 | `block`, `number-box` | One stone of the builder's tower. The code stacks them and writes the number. |
| 5 | `highlights.svg` | 400 × 400 | `circle`, `underline`, `arrow`, `bracket` | Strokes only, Pencil tool, one motion each. The code stretches each to fit what it marks (addends, the `=` sign, the answer). |
| 6 | `ada.svg` | 300 × 300 | `ada`, `eyes-open`, `eyes-closed`, `mouth-closed`, `mouth-open` | Narrator portrait. The code blinks her and opens her mouth while the voice plays. |
| 7 | `hopper.svg` | 60 × 60 | `hopper` | Small figure that stands on the number line and hops. |
| 8 | Paper texture | 512 × 512 | none | Photograph or scan real paper (one Gentle, one Exceptional). PNG/JPG, must tile without seams. This beats drawn grain. |
| 9 | `cover.svg` | 600 × 900 | `art`, `title` (hand-lettered "The Primer"), `name-box` (empty, the code writes the hero's name) | The first thing anyone sees. |
| 10 | `chapter-1.svg`, `chapter-2.svg` | 600 × 400 | `art` | Chapter openers: the tower or machine, then the pouches. |
| 11 | `divider.svg` | 300 × 40 | `divider` | Ornament between sections. |
| 12 | `builder.svg` | 300 × 300 | `builder` | The builder from the word problems. |
| 13 | `proved-exceptional.svg` | 200 × 200 | `seal` | Seal for the Exceptional track. |
| 14 | `marks.svg` | 100 × 100 | `tick` (ink), `cross` (`mark` red) | Beside a checked answer. Pencil strokes: the tick in one motion, the cross in two. A code-drawn placeholder is in `assets/lessons/marks.svg`. |
| 15 | `deckle.svg` | 150 × 150 | one path | The torn page edge of the Gentle look (`assets/textures/deckle.svg`, a code-drawn placeholder). Keep the torn outline inside the outer 30 units, filled with `paper`, and the same shape at both ends of each side so the repeats join. Used as `border-image` (see `docs/FRONTEND.md`). |

Item 5 (`highlights.svg`) also draws the underline under every "Next" / "Continue" button (layer `underline`).

Items 1–7 are needed for a demo of chapters 1 and 2; 8–9 make it feel like a
book; 10–13 can wait.

## 4. Checking your drawing

Save into `art/`, then tell Michael (god). An agent wires it in and you check
it in the running prototype on your phone.
