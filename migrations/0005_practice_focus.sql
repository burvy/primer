-- "More practice": when a child asks for extra practice on a chapter from the
-- contents page, the skill is remembered here, and the book keeps giving
-- generated problems for it until they go back to the main path. NULL is the
-- main path.
ALTER TABLE children ADD COLUMN practice_skill TEXT;
