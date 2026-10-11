# UI audit (step 1): what looks unfinished

Branch `slice-ui-audit`, from main `7bc47ca`. No code or CSS was changed.
The bar: a hand-made storybook (Gentle) or Byrne's Euclid treatise
(Exceptional), never a corporate web app.

## How to read the screenshots

Names are `NN-screen-size[-look].png`.

- **Sizes:** `phone` is 390 px wide (drawn at 2x), `laptop` is 1280 x 800.
  Phone shots are the whole page height (the page is taller than the screen
  where you see more than 844 px).
- **Looks:** `gentle` / `exceptional`. Screens 01 to 11 (sign-in, grown-ups,
  profiles, hero picker, privacy) have **no look suffix**: they are always
  drawn in Gentle, even for a child whose look is Exceptional.
- **Source of the data:** a fresh server (`PRIMER_DEV_SIGNIN=1`), two readers
  (one per look), played by a script that answers like a child: one wrong
  answer first, then right ones, until the end of the book. Drawings are the
  placeholders now in `assets/` (the machine, the pouch, the tower); Ada is the
  grey silhouette; there are no recordings played (silent capture).
- **Not captured:** Google's sign-in button (needs a client id); a chapter-2
  opener (chapter 2 has none yet); `counters` as a lesson picture (no lesson
  uses it); "More practice" (it looks like the practice screens).

| # | screen | files |
|---|---|---|
| 01 | sign-in | `01-sign-in-{phone,laptop}.png` |
| 02 | privacy | `02-privacy-*` |
| 03 | who is reading, no reader yet | `03-profiles-empty-*` |
| 04, 05 | grown-ups gate; wrong answer | `04-grown-ups-gate-*`, `05-...-wrong-*` |
| 06 | parent page, empty | `06-parent-empty-*` |
| 07, 08 | parent page with readers; "Really delete?" armed | `07-parent-with-readers-*`, `08-parent-delete-armed-*` |
| 09 | who is reading | `09-profiles-*` |
| 10, 11 | hero picker; a refused name | `10-hero-picker-*`, `11-hero-picker-refused-*` |
| 12 | cover | `12-cover-*-{gentle,exceptional}` |
| 13 | chapter opener (ch. 1) | `13-chapter-opener-*` |
| 14-18 | lessons: tower, number-line, equation, machine, machine (later beat) | `14`..`18-lesson-*` |
| 19 | practice, word problem (story + ten frame) | `19-practice-word-problem-*` |
| 20, 21 | practice, empty keypad; typed answer | `20-practice-empty-*`, `21-practice-typed-*` |
| 22, 23 | Ada asks "how did you work it out"; one picked | `22-explain-question-*`, `23-...-picked-*` |
| 24 | Ada asks "how are you feeling" | `24-feel-question-*` |
| 25 | hint (a wrong answer) | `25-feedback-hint-*` |
| 26 | rest | `26-rest-*` |
| 27 | contents (part-way) | `27-contents-*` |
| 28 | practice, ten frame | `28-practice-ten-frame-*` |
| 29, 30 | set notice; set question | `29-set-notice-*`, `30-set-question-*` |
| 31 | praise | `31-feedback-praise-*` |
| 32 | practice, number line | `32-practice-number-line-*` |
| 33, 34 | lesson, two machines; a later beat | `33-lesson-machines-*`, `34-...-later-*` |
| 35 | practice, no picture | `35-practice-none-*` |
| 36 | lesson, pouches | `36-lesson-pouches-*` |
| 37 | end of book | `37-end-of-book-*` |
| 38 | contents at the end | `38-contents-end-*` |

Not drawn as their own screens, because they are one component in many
screens: the **toolbar** at the top of every book page (look button, Voice /
Sounds / Music, Contents, Grown-ups) and **Ada**.

## What looks unfinished, by screen

Four problems come up on nearly every screen; they are listed once here and
then not repeated.

- **A. The same grey-card frame everywhere.** Every page is one rounded
  rectangle with a thin border and a soft shadow, on a flat beige background
  (`.book` in `style.css`). That is how a web app draws a card. A book page
  has no border radius and no float: it has a margin, a printed block and
  a running head.
- **B. Web-app chrome on every book page.** The toolbar at the top (a look
  button, three tick boxes, two underlined links floated right) sits above
  every lesson, problem and verdict. On a phone it takes about 130 px and wraps
  to three rows (`12-cover-phone-gentle`). It reads like settings, not a
  page. The look button is the largest thing on the cover.
- **C. Placeholder art.** Ada is a grey silhouette in a circle; the machine is
  a dark block with a white rectangle; the page "texture" is a few faint
  dots (Gentle) or a ruled grid (Exceptional). `.ornament` (the divider) and
  the Exceptional seal exist as files and CSS variables but **appear on no
  screen**.
- **D. Everything is the same button.** Next, Check, Continue, Begin, Open the
  book, Sign in, Add reader, Back to reading: one rounded brown pill. "Next"
  and "Check" are the same weight as "Sign out". The quiet actions are blue
  or grey underlines.

### Sign-in, privacy, profiles, grown-ups (01 to 11)

- **01 sign-in.** Small card in a big empty page; the title is the same size
  as a form heading, with no cover feel. The empty error line (`.note.error`
  reserves 1.5 em) leaves an unexplained gap above "Privacy". The local test
  button is correct for now; the Google button could not be captured.
- **02 privacy.** A bulleted list inside a centred panel: the bullets stay on
  the left and the lines are centred, so each bullet is ragged both sides and
  hard to read (`02-privacy-phone`). Long text wants left-aligned, a narrower
  line, and spacing between items.
- **03 / 09 profiles.** The two readers' buttons are both "New reader"
  (before a hero name is chosen) and nothing tells them apart (look, age).
  The "grown-up" in the empty message breaks across lines at its hyphen
  ("grown-/up"). The page is a tiny island in the viewport.
- **04 / 05 gate.** Fine in structure. The input is a native text box and
  the Go-on button sits under it with a different width; "Not quite. Try
  this one." appears with no gap rule. Laptop shows the input narrow and
  centred, which is good.
- **06 to 08 parent.** The headings "Readers" and "Your account" use the same
  weight as "Grown-ups": no hierarchy. Empty "Readers" has no message at all
  (`06-parent-empty-phone`). A reader reads as a sentence with `·` separators
  that wraps unpredictably on phone ("(no hero name yet) · age 7-8 ·
  exceptional look  Export  Delete"). "Save the export file" is a **default
  blue browser link**, the only off-palette colour on any screen (`07`).
  The age and look pickers are native `<select>`s, which draw in the operating
  system's style and wrap onto two rows. "Delete" and "Delete my account" are
  link-sized next to big brown buttons.
- **10 / 11 hero picker.** 33 names in a 2-column list is **2,800 px tall on a
  phone** (`10-hero-picker-phone`): the "type your own" box is far below the
  fold. All names look like equal buttons, so nothing invites a pick. "al-
  Khwarizmi" breaks at its hyphen on laptop. The refused-name message is small
  red text without a frame, below the button.

### The book: cover, openers, lessons (12 to 18, 33, 34, 36)

- **12 cover.** The cover is the one place to feel like a book and is
  typeset text on a card: "The Primer / for / Euclid / Open the book". The
  toolbar sits above the title (B). On laptop the card sits at the top of an
  empty page (not centred).
- **13 opener.** Number and title in a small block with the Begin button
  directly under it, no picture, no space; it is the weakest "title page".
- **14 to 17 lessons.** Ada is a left-aligned grey circle on its own row,
  then a large gap, then the picture, then the text. On phone there is no
  relationship between Ada and her words (she is a spectator above the
  picture). The picture area has no frame or caption, so numbers and text
  float.
- **15 number line.** On phone it is drawn about 270 px wide, its numerals
  about 10 px tall: below a comfortable size for a 5-year-old.
- **17 machine (Exceptional).** The single machine's drawing keeps its flat
  background rectangle, which shows as a lighter slab against the textured
  page in Exceptional (`17-lesson-machine-phone-exceptional`). The joined
  machines (33) correctly hide it.
- **18 / 34 machine numbers.** "in: 4 -> out: 5" and "machine 1: in: 3 -> out: 4"
  are plain grey centred text in the maths font; they look like debug output,
  not labels on the machine.
- **36 pouches.** The pouches sit at the bottom of a tall drawing area with
  about 150 px of empty space above (`36-lesson-pouches-phone-gentle`).
- **Motion.** Every beat plays a page-turn on the picture and a separate one
  on the text; both are quick (350 ms) and fine. The machine run (1.4 s) and
  the hand-drawn highlight strokes are not visible in a still: check them in
  the browser.

### Practice and Ada's replies (19 to 32, 35)

- **19 / 20 / 35 the equation.** The answer box is an underlined gap that sits
  **lower and to the right** of "3 + 1 =" instead of on its baseline
  (`20-practice-empty-phone-gentle`): the underline drops under the text and
  looks detached. The same on laptop.
- **Keypad.** Keys are 56 px, centred in a column about 184 px wide on a 390 px
  screen: room for bigger keys. The grid has a hole where a fourth key would
  be (bottom right). "I don't know" and "Check" stack below at different
  widths. Disabled "Check" is a washed-out brown (opacity 0.4): readable but
  looks broken, not waiting.
- **Word problem (19).** Ada's story, a ten-frame picture, the equation and
  the keypad make four stacked blocks on phone; the page is ~940 px tall,
  so Check is below the fold on a small phone.
- **21 typed.** The typed digit appears above the underline and not on the
  line (same alignment problem).
- **22 to 24 questions.** The strongest screens: big picture buttons, a clear
  picked state. The "picked" state is a thicker border and a slightly darker
  fill, easy to miss at arm's length; Done is grey until a pick. Emoji are
  full-colour system pictures, outside the palette (they will look different
  on each device). Skip and Done are different sizes and the order puts the
  skip first.
- **25, 26, 29 to 31 Ada's lines.** Ada beside one short paragraph, then a
  button; a lot of empty page below. There is no right/wrong signal other than
  the words: no colour, no mark, and the hint does not show the problem it is
  about. On phone Ada's line is a narrow column beside her.
- **28 ten frame / 32 number line (practice).** Fine, but small on phone; the
  number-line hop arc is tiny.

### Contents, end of book (27, 37, 38)

- **27 / 38 contents.** A flat list of nine lines per chapter. Lessons are
  underlined links, practices and stories are plain text, the new lesson is bold, the
  ticks are small and the same grey as the words. Hard to tell what you can
  tap. "you are here" is tiny. The chapter heading is red for the current
  chapter and black for the rest, which reads as an error colour.
  **Possible content bug:** at the end (`38`) chapter 2 says "finished" while
  Lessons 3 and 4 and Practices 3 and 4 have no tick and "Practice 2" says
  "2 of 4". The toc marks a chapter done when the skill is mastered
  (`content/toc.rs`), not when every step was read; the child can see that as
  a contradiction. Worth a decision (see the options below).
- **37 end of book.** A heading, one sentence and a button on an empty card.
  It is the last page of the prototype and has no picture, no ornament.

## Ranked fixes, in three batches

Effort: S = under an hour of CSS or one component, M = a day, L = needs the
human's art or a decision first.

### Batch 1 (S): quick wins, no taste decision

1. **The answer box underline** sits below the equation line. Fix the box's
   alignment (`.equation .box`). `20`, `21`, `35`.
2. **Left-align the privacy list**, set a reading width and item spacing. `02`.
3. **Style `a` links** with the palette (`Save the export file`), and the
   native `<select>` / `<input>` consistently. `07`.
4. **Hero picker**: put "type your own" **above** the 33 names, or show names
   in a smaller, denser grid; stop names breaking at their hyphen
   (`white-space: nowrap`). `10`, `11`.
5. **Empty-state wording and spacing**: a line under "Readers" when there are
   none; remove the unexplained gap above "Privacy"; give each reader button
   its age and look. `01`, `03`, `06`, `09`.
6. **Keypad**: keys 64 to 72 px, fill the width, fix the empty slot (move
   "I don't know" into it). `20`.
7. **Machine numbers** ("in: 3 -> out: 4"): give them a small label style. `18`, `34`.
8. **Hide the single machine's background rectangle** in Exceptional like the
   joined view does. `17`.
9. **Centre the card on laptop**, or let the cover and openers fill the height.
   `12`.

### Batch 2 (M): structural, small decisions

10. **Move the toolbar out of the page.** Collapse look/sound/contents/grown-ups
    into one quiet running head (a single line: "Contents · Grown-ups" at the
    top corner, the look and sound switches behind one "..."). Decision
    needed: see taste choice 2. `12` to `38`.
11. **Place Ada with her words** on lessons (beside the text, like practice),
    so she is the speaker, not a spectator. `14` to `18`.
12. **Real hierarchy in the grown-up pages**: sections with rules between
    them, readers as a small table or cards. `06` to `08`.
13. **Distinguish the buttons**: the main action (Next/Check/Continue) vs
    secondary (Skip, Back, I don't know) vs dangerous (Delete). Taste choice 1.
14. **Right/wrong signal** on the verdict page: a mark (a tick or a cross in
    ink or red) and the equation shown with the answer written in. `25`, `31`.
15. **Contents list**: one consistent row design with a tick column, and a
    "you are here" marker that is not red text. Resolve "finished" while items
    are not ticked. `27`, `38`.
16. **Bigger number line** on phone, with larger numerals. `15`, `32`.

### Batch 3 (L): needs art or a decision

17. **Ada, the machine, pouches, tower stones**: replace placeholders with the
    hand-drawn files (`docs/ART_GUIDE.md`). Nothing in the code has to change.
18. **Paper**: a real scanned paper texture and a real page edge. Taste
    choice 3.
19. **A real cover and chapter openers** (`cover.svg`, `chapter-N.svg`), and
    use the ornament divider and the Exceptional seal, which exist but are never
    shown.
20. **Emoji for the feelings**: replace with drawn faces in the palette, so
    they look the same everywhere.
21. **Sign-in screen** as a title page of the book (art, name, one button).

## Taste choices for the human

These are your decisions; the code can do any of them.

1. **Button style.** (a) *Ink-stamped label*: a filled rectangle with a
   slightly rough edge, like a printed button or label. (b) *Underlined
   text*: no box at all, a word with a hand-drawn underline stroke (we
   already have `highlights.svg`), the way a book prints "turn the page".
   (c) *Drawn frame*: a thin hand-drawn box round the word; the Primary one
   gets a second line or a colour wash. Today every action is the filled
   brown pill.
2. **The toolbar.** (a) *Running head*: one line at the top, small caps, in
   book style ("Contents · Grown-ups"), look and sound behind a small
   "Settings" mark. (b) *Margin tabs*: a little bookmark ribbon at the edge
   of the page for Contents, a corner fold for settings. (c) *Hidden until
   asked*: only a small corner mark; tap it to open the list.
3. **The page frame.** (a) *Plain*: a paper block with a margin and no border
   (the page is the screen). (b) *Deckle edge*: an irregular, torn edge
   drawn in SVG, with a soft shadow. (c) *Ruled margin*: a thin vertical
   rule down the left, like an exercise book (or Byrne's thin border for
   Exceptional).
4. **Ada's place on phone.** (a) A small portrait in the corner with the
   line beside it (like practice today). (b) A larger portrait above, the
   line in a speech-scroll under it. (c) A margin note: Ada small at the
   edge of the text.
5. **Right and wrong.** (a) A drawn tick or cross beside the answer. (b)
   Only Ada's words (today). (c) A colour wash on the equation (palette
   green is not in the palette, so this would need the human to add one).
6. **Chapters "finished" while steps are not read** (contents, 38). (a) Show
   "finished" only when every step has been read. (b) Keep it as mastery and
   write "you did it, skipped some". (c) Mark the unread steps "not
   needed".

## Findings for other people

- Accessibility: contrast of `ink-soft` (`#7a6450`) on `paper` is about 4.6 to 1:
  passes for normal text, but not by much; do not make it smaller than
  today's 0.9 rem (the Music label and the progress notes).
- No code paths are broken in the screens above; every screen opened and
  closed correctly in both looks and at both sizes.
