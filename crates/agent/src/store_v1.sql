CREATE TABLE tasks (
    task_id TEXT PRIMARY KEY NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    recorded_state TEXT NOT NULL CHECK (recorded_state IN ('running', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown')),
    last_seq INTEGER NOT NULL CHECK (last_seq > 0),
    artifact_created INTEGER CHECK (artifact_created IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
CREATE TABLE events (
    event_id TEXT UNIQUE NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    seq INTEGER NOT NULL CHECK (seq > 0),
    event_type TEXT NOT NULL CHECK (event_type IN ('started', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (task_id, seq)
) STRICT;
