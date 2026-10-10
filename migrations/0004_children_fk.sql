-- Children belong to a parent, and deleting a parent or a child deletes
-- everything below it (`ON DELETE CASCADE`): the child's attempts go with the
-- child, the children with their parent. A child profile is a hero name (a
-- picked or checked name, never a real one), an age band and a look.
--
-- SQLite cannot add a foreign key to an existing table, so both tables are
-- rebuilt: make the new one, copy the rows, drop the old one, rename.
--
-- Children made before sign-in existed (the old built-in `dev` child) go to
-- one parent called 'dev-local', so no row is lost.

INSERT INTO parents (google_sub, created_at)
SELECT 'dev-local', 0 WHERE EXISTS (SELECT 1 FROM children);

CREATE TABLE children_new (
    id         TEXT PRIMARY KEY,
    parent_id  INTEGER NOT NULL REFERENCES parents(id) ON DELETE CASCADE,
    hero_name  TEXT,
    age_band   TEXT NOT NULL DEFAULT '5-6',
    look       TEXT NOT NULL,
    seed       INTEGER NOT NULL,
    narration  INTEGER NOT NULL DEFAULT 1,
    sounds     INTEGER NOT NULL DEFAULT 1,
    music      INTEGER NOT NULL DEFAULT 0
);
INSERT INTO children_new (id, parent_id, look, seed, narration, sounds, music)
SELECT id, (SELECT id FROM parents WHERE google_sub = 'dev-local'), look, seed,
       narration, sounds, music
FROM children;

CREATE TABLE attempts_new (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    child_id TEXT NOT NULL REFERENCES children_new(id) ON DELETE CASCADE,
    kind     TEXT NOT NULL,
    data     TEXT NOT NULL
);
INSERT INTO attempts_new (id, child_id, kind, data)
SELECT id, child_id, kind, data FROM attempts;

DROP TABLE attempts;
DROP TABLE children;
ALTER TABLE children_new RENAME TO children;
ALTER TABLE attempts_new RENAME TO attempts;
CREATE INDEX attempts_by_child ON attempts (child_id, id);
