CREATE TABLE tasks_v7 (
    task_id TEXT PRIMARY KEY NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 3 CHECK (schema_version IN (2, 3)),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    recorded_state TEXT NOT NULL CHECK (recorded_state IN ('running', 'waiting_approval', 'cancel_requested', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown')),
    last_seq INTEGER NOT NULL CHECK (last_seq > 0),
    artifact_created INTEGER CHECK (artifact_created IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
INSERT INTO tasks_v7 SELECT * FROM tasks;
CREATE TABLE events_v7 (
    event_id TEXT UNIQUE NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks_v7(task_id),
    schema_version INTEGER NOT NULL DEFAULT 4 CHECK (schema_version IN (1, 2, 3, 4)),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    seq INTEGER NOT NULL CHECK (seq > 0),
    event_type TEXT NOT NULL CHECK (event_type IN ('started', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown', 'approval_requested', 'approval_granted', 'approval_denied', 'approval_expired', 'approval_revoked', 'tool_dispatched', 'tool_succeeded', 'tool_failed', 'tool_rejected', 'tool_outcome_unknown', 'cancel_requested')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    related_id TEXT,
    PRIMARY KEY (task_id, seq)
) STRICT;
INSERT INTO events_v7 SELECT * FROM events;
DROP TABLE events;
DROP TABLE tasks;
ALTER TABLE tasks_v7 RENAME TO tasks;
ALTER TABLE events_v7 RENAME TO events;
