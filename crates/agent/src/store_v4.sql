CREATE TABLE sessions (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(task_id),
    generation INTEGER NOT NULL CHECK (generation > 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    session_json TEXT NOT NULL
) STRICT;
