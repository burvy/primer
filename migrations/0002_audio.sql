-- What the child wants to hear. Stored like `look`, so it follows the child
-- across devices. 1 is on, 0 is off. Narration and sounds start on, music off.
ALTER TABLE children ADD COLUMN narration INTEGER NOT NULL DEFAULT 1;
ALTER TABLE children ADD COLUMN sounds INTEGER NOT NULL DEFAULT 1;
ALTER TABLE children ADD COLUMN music INTEGER NOT NULL DEFAULT 0;
