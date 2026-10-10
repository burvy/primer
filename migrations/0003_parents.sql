-- Parents sign in with Google. We keep only Google's permanent id for the
-- account (`google_sub`): no email, no name. The `dev-local` sign-in (used
-- when no Google client id is set) is the parent whose `google_sub` is
-- 'dev-local'.
CREATE TABLE parents (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    google_sub TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);

-- A sign-in. `token` is random and secret; the browser sends it as
-- `Authorization: Bearer <token>`. `expires_at` is seconds since 1970. Deleting
-- a parent deletes their sessions.
CREATE TABLE sessions (
    token      TEXT PRIMARY KEY,
    parent_id  INTEGER NOT NULL REFERENCES parents(id) ON DELETE CASCADE,
    expires_at INTEGER NOT NULL
);
