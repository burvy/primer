# Review of Draft Chapters 3–9

Review against target voice in `01-count-on.toml` and `02-add.toml` (WRITERS_GUIDE rules: <12 words/lesson line, <20 words/word problem, matching `show` components, no formal algebra for ages 5–8, promised problem shapes).

## Chapter 3: `03-missing-addend.toml` (Missing Addend within 10)
- **Wrong maths / promise violations**:
  - Top comment promises "Every set hides the second number", but `ch3-family-walk` problems include `"3 + 4"` and `"4 + 3"` (neither hides a number) and `"□ + 4 = 7"` (hides first number).
  - `ch3-lesson` s2 b1 introduces subtraction (`$7 - 3 = 4$`) before subtraction is taught (Chapter 5).
  - `ch3-pouch` hardcodes `7` in text while `a = 3, b = 4` (breaks `{a}` generator slot); `ch3-steps` hardcodes `step 9` with `a = 4, b = 5`.
- **Words/ideas 5–8 year old would not know**:
  - `ch3-lesson` s2 b0 formal uses variable `x` (`$3 + x = 7$`); `ch3-missing-row` formal_notice uses formula `$x = c - a$`.
  - `ch3-hint-off-by-one` / `ch3-hint-wrong-operation` use undefined abstract terms `"the whole"` and `"the gap"`.
- **Line does not match picture (`show`)**:
  - `ch3-lesson` s0 b0 (`show = "pouches"`, `left = 3, right = 4`): renders 4 coins in the right pouch, revealing the hidden answer for `3 + □ = 7`.
- **Repeated lines across chapters**:
  - `ch3-hint-off-by-one-line`: `"Stand on the first number. Count only the hops."` (exact match with `ch1-hint-off-by-one-line`).

## Chapter 4: `04-make-ten.toml` (Make Ten)
- **Wrong maths / promise violations**:
  - `ch4-compensation`: claims to show compensation ("move one from hiding pile to seen pile"), but problems are four standalone equations (`["3 + □ = 10", "4 + □ = 10", ...]`) without paired shifts.
  - `ch4-bond-walk` formal_notice: cites bonds as addition pairs (`$1 + 9$ through $5 + 5$`), but problems are missing-addend (`"1 + □ = 10"`).
- **Words/ideas 5–8 year old would not know**:
  - `ch4-lesson` s2 b1, `ch4-compensation` notice, `ch4-praise-3`: `"compensation"` (heavy formal term; compare `02-add.toml` which glosses "swap rule" alongside "commutativity").
  - `ch4-gate` formal uses variable `x` (`${a} + x = 10$`); `ch4-hint-off-by-one-line` formal uses formula `$10 - a$`.
- **Line does not match picture (`show`)**:
  - `ch4-lesson` s0 b0: text says `"Two pouches must fill one ten frame"`, but `show = "pouches"` draws money pouches with `right = 7` (spoils answer), not a ten-frame.
  - `ch4-hint-off-by-one-line` (`stage = "number-line"`): text refers to `"empty slots in the frame"`.
  - `ch4-hint-off-by-one-pouches` (`stage = "pictorial"`): text refers to `"filled slot"` and `"empty one"`, not pouches.
- **Repeated lines across chapters**:
  - `ch4-gate`: `"The builder stands on step {a}..."` repeats staircase formula from `ch3-steps`, `ch6-stairs`, `ch8-steps`.

## Chapter 5: `05-subtract.toml` (Subtract within 10)
- **Wrong maths / promise violations**:
  - `ch5-predecessor-walk`: top comment says "answers fall by one each time", but `problems` are `["7 - 1", "7 - 2", "7 - 3", "7 - 4"]` (subtracting increasing amounts, not finding predecessors of 7).
  - `ch5-undo-pairs`: describes "Undo pairs: add, then take back", but `problems` has 4 subtractions and zero additions (`["7 - 4", "7 - 5", "6 - 2", "6 - 3"]`).
- **Words/ideas 5–8 year old would not know**:
  - `ch5-lesson` s2 b2: `"They are inverse."` (unglossed; compare `02-add.toml` which explains inverses in context).
- **Line does not match picture (`show`)**:
  - `ch5-lesson` s0 b0/b1 (`show = "pouches"`): `left = 7, right = 3` then `left = 4, right = 3` renders two separate pouches, not coins leaving a single pouch.
  - `ch5-lesson` s1 b1 (`show = "number-line"`, `at = 4`): missing `hop` parameter; web `NumberLine` hardcodes forward hops only (`forward=true`).
- **Repeated lines across chapters**:
  - `ch5-predecessor-walk` question: `"Look at the answers. What do you notice?"` (identical to `ch2-doubles-set`).
  - `ch5-tower` formal: `"A tower of {a} steps loses {b}. Find ${a} - {b}$."` (word-for-word identical to `ch7-tower`).
  - `ch5-hint-wrong-operation`: `"Look at the sign. $-$ means take away."` (word-for-word identical to `ch7-hint-wrong-operation`).

## Chapter 6: `06-add-tens.toml` (Add within 20, No Carrying)
- **Wrong maths / promise violations**:
  - `ch6-parallel-small`: description promises "small sum, then the same sum with a ten", but problems only list teen sums (`["13 + 4", "14 + 3", ...]`). The small twins (`3 + 4`) are omitted.
  - `ch6-stairs`: text hardcodes `"step 12"` instead of `{a}` slot (`a = 12, b = 5`).
- **Words/ideas 5–8 year old would not know**:
  - `ch6-lesson` s2 b1: abstract term `"place value"` introduced without concrete visual packaging.
  - `ch6-hint-off-by-one` formal: refers to `"carries over"`, contradicting the chapter's "no carry" concept.
- **Line does not match picture (`show`)**:
  - `ch6-lesson` s0 b0/b1 (`show = "counters"`, `count = 13/17`): text describes a `"full frame and 3 loose counters"`, but `Counters` draws a single flat horizontal row of dots, not a ten-frame.
- **Repeated lines across chapters**:
  - `ch6-parallel-bare` question: `"Cover the tens digit. What do you see?"` (identical to `ch7-parallel-bare`).
  - `ch6-stairs` formal: `"From step {a}, climb {b} more. Find ${a} + {b}$."` (identical to `ch8-steps`).

## Chapter 7: `07-subtract-tens.toml` (Subtract within 20, No Borrowing)
- **Wrong maths / promise violations**:
  - `ch7-parallel-small`: promises parallel small/teen pairs, but only contains teen subtractions (`["17 - 3", "16 - 2", ...]`).
  - `ch7-parallel-bare` notice: says `"ones sums stay small"` in a subtraction chapter.
  - `ch7-counters`: text hardcodes `"16 counters"` and formal hardcodes `"From 16 counters"` instead of `{a}` slot (`a = 16`).
- **Words/ideas 5–8 year old would not know**:
  - `ch7-lesson` s2 b0 formal: `"7 ones minus 3 ones is 4 ones."` (awkward phrasing for young readers).
- **Line does not match picture (`show`)**:
  - `ch7-lesson` s0 b0/b1 (`show = "counters"`): text says `"One full frame and 7 loose counters"`, but `counters` renders a plain dot row.
  - `ch7-lesson` s1 b1 (`show = "number-line"`, `at = 14`): missing `hop` parameter; backward hop unsupported in lesson number line.
- **Repeated lines across chapters**:
  - `ch7-tower` text/formal: nearly identical copy of `ch5-tower` (`"The wind takes {b} down"` / `"A tower of {a} steps loses {b}"`).
  - `ch7-hint-wrong-operation`: exact match of `ch5-hint-wrong-operation`.

## Chapter 8: `08-bridge.toml` (Add within 20, Crossing Ten)
- **Wrong maths / promise violations**:
  - `ch8-bridge-pairs`: claims "make ten first, then the real sum", but problems only contain the final sums (`["8 + 5", "7 + 6", "6 + 9", "9 + 5"]`); missing the make-ten step (`8 + 2`).
  - `ch8-bridge-line` formal_notice: cites `"$9 + 4 = 10 + 3 = 13$"`, but `9 + 4` is not in the problem set (`["9 + 5", "9 + 6", "6 + 7", "6 + 9"]`).
- **Words/ideas 5–8 year old would not know**:
  - `ch8-hint-carry-or-borrow` formal: `"Split the addend"`.
  - `ch8-hint-off-by-one-line` formal: `"hop the remainder"` (division terminology).
- **Line does not match picture (`show`)**:
  - `ch8-lesson` s0 b0/b1/b2 (`show = "counters"`): text describes a frame of 8, moving 2 over to fill 10, but `counters` has no frame or partitioning.
  - `ch8-lesson` s1 b1 (`show = "number-line"`, `at = 10, hop = 3`): teleports to 10 and hops 3, omitting the initial hop from 8 to 10.
  - `ch8-pouches`: uses `show = "pouches"` for word problem, abandoning the counters/frame metaphor of the lesson.
- **Repeated lines across chapters**:
  - `ch8-bridge-line` question: `"Where do you stop before finishing?"` (identical to `ch9-bridge-back-line`).
  - `ch8-steps` formal: exact match with `ch6-stairs`.
  - `ch8-hint-wrong-operation` formal: `"The sign is $+$: expect a bigger number."` (mirror copy in ch9).

## Chapter 9: `09-bridge-back.toml` (Subtract within 20, Crossing Ten)
- **Wrong maths / promise violations**:
  - `ch9-bridge-back-pairs`: claims "land on ten first, then the real take-away", but problem list omits the landing sums (has `["13 - 5", "14 - 6", ...]` without `13 - 3`).
  - `ch9-counters`: hardcodes `"13 counters"` in text and formal instead of using `{a}` slot (`a = 13, b = 5`).
  - `ch9-tower`: hardcodes `"15 steps"` in text and formal instead of using `{a}` slot (`a = 15, b = 7`).
- **Words/ideas 5–8 year old would not know**:
  - `ch9-hint-carry-or-borrow` formal: `"Borrow from the ten"` (column subtraction jargon; this chapter is mental bridging back).
  - `ch9-hint-off-by-one-line` formal: `"hop the remainder"`.
- **Line does not match picture (`show`)**:
  - `ch9-lesson` s0 b0/b1/b2 (`show = "counters"`): text refers to full frame and breaking into it, but `counters` is a plain row.
  - `ch9-lesson` s1 b1 (`show = "number-line"`, `at = 10`): missing `hop` parameter; backward hop unsupported in lesson component.
- **Repeated lines across chapters**:
  - `ch9-bridge-back-line` question: identical to `ch8-bridge-line`.
  - `ch9-tower` text/formal: third copy of `ch5-tower` / `ch7-tower`.
  - `ch9-hint-off-by-one-line` formal: identical structure to `ch8-hint-off-by-one-line`.
