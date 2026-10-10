# Primer writer's guide (draft for the human to edit).
#
# How to write a chapter: one lesson, then ordered problem sets, then the
# small extras. Every line below is a rule the loader and lint will check.

# 1. One file per chapter: `content/01-count-on.toml`, `content/02-add.toml`.
#    Copy `content/_template.toml` and fill it in. File must parse as TOML.

# 2. Lesson first. Ada walks ONE worked example through the three stages:
#    pictorial (counters), number line, bare equation. Name each chapter word
#    once, with its Gentle gloss beside it. Short sentences. No baby talk.

# 3. Problem sets are ordered, never random. Each set picks one pattern
#    (chapter 1: successor; chapter 2: successor, swap, doubles), lists 3-6
#    problems in that order, and ends with a `notice` line where Ada names
#    the idea ("That is the *commutative* rule.").

# 4. Every problem keeps its chapter's promise. Chapter 1: sums up to 5
#    ("3 + 1", never "3 + 4"). Chapter 2: sums up to 10 ("3 + 4", never
#    "8 + 5"). Write "a + b" for answer-known, "a + box = c" for missing.
#    The box sign is `□` (U+25A1). No other digits may appear in text.

# 5. Word problems use {a} and {b} slots ("{a} ducks, {b} more join."). Hints
#    come one per mistake kind (off-by-one, wrong-operation, digits-swapped,
#    carry-or-borrow, guess). Praise is one short line ("Clean reasoning.").

# 6. Reading level: ages 5-8. One idea per sentence. Under 12 words per line
#    for lessons, under 20 for word problems. Gentle shows the gloss first
#    ("the next number"); Exceptional leads with the formal word ("successor").

# 7. Voices: Ada narrates (warm, exact, speaks to a capable reader). The hero
#    is the child (picked from `hero_names.toml`, never written into chapters).
#    No mascots, no real names, no Stephenson names.

# 8. Each entry needs id, skill, stage (pictorial, number-line, abstract),
#    and look (gentle, exceptional, both; default both). Ids are unique across
#    all files. Every skill needs a lesson and at least one written set.

# 9. Lint checks all of this (ids, slots, word limits, chapter promises, word
#    and hint coverage) plus Jev checks (reading level, no answer given away,
#    kid-safe). The server refuses broken content. Fix the file, not the test.
