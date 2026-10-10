-- One row per child. `look` is 'gentle' or 'exceptional'; `seed` makes the
-- child's generated sets repeatable.
CREATE TABLE children (
    id   TEXT PRIMARY KEY,
    look TEXT NOT NULL,
    seed INTEGER NOT NULL
);

-- An append-only log: rows are only ever added, never edited. The Learner is
-- not stored anywhere; it is folded from these rows each time it is needed
-- (docs/DESIGN.md). `kind` is 'lesson' or 'attempt'; `data` is a JSON object.
-- Lessons share the table because their order relative to attempts matters.
CREATE TABLE attempts (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    child_id TEXT NOT NULL REFERENCES children(id),
    kind     TEXT NOT NULL,
    data     TEXT NOT NULL
);
CREATE INDEX attempts_by_child ON attempts (child_id, id);
