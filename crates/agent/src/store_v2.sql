CREATE TABLE tasks_v2 (
    task_id TEXT PRIMARY KEY NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 2 CHECK (schema_version = 2),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    recorded_state TEXT NOT NULL CHECK (recorded_state IN ('running', 'waiting_approval', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown')),
    last_seq INTEGER NOT NULL CHECK (last_seq > 0),
    artifact_created INTEGER CHECK (artifact_created IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
INSERT INTO tasks_v2 SELECT task_id, 2, generation, recorded_state, last_seq, artifact_created, updated_at FROM tasks;
CREATE TABLE events_v2 (
    event_id TEXT UNIQUE NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks_v2(task_id),
    schema_version INTEGER NOT NULL DEFAULT 2 CHECK (schema_version IN (1, 2)),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    seq INTEGER NOT NULL CHECK (seq > 0),
    event_type TEXT NOT NULL CHECK (event_type IN ('started', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown', 'approval_requested', 'approval_granted', 'approval_denied', 'tool_dispatched', 'tool_succeeded', 'tool_failed')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    related_id TEXT,
    PRIMARY KEY (task_id, seq)
) STRICT;
INSERT INTO events_v2 SELECT event_id, task_id, schema_version, generation, seq, event_type, created_at, NULL FROM events;
DROP TABLE events;
DROP TABLE tasks;
ALTER TABLE tasks_v2 RENAME TO tasks;
ALTER TABLE events_v2 RENAME TO events;
CREATE TABLE task_specs (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(task_id),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    spec_revision INTEGER NOT NULL DEFAULT 1 CHECK (spec_revision > 0),
    policy_revision INTEGER NOT NULL DEFAULT 1 CHECK (policy_revision > 0),
    spec_json TEXT NOT NULL,
    spec_hash TEXT NOT NULL
) STRICT;
CREATE TABLE tool_intents (
    intent_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    generation INTEGER NOT NULL CHECK (generation > 0),
    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL CHECK (tool_name IN ('read_input', 'write_output')),
    tool_version INTEGER NOT NULL CHECK (tool_version = 1),
    args_json TEXT NOT NULL,
    args_hash TEXT NOT NULL,
    scope_path TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('prepared', 'denied', 'dispatched', 'succeeded', 'failed', 'cancelled')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (task_id, generation, call_id)
) STRICT;
CREATE TABLE approvals (
    approval_id TEXT PRIMARY KEY NOT NULL,
    intent_id TEXT UNIQUE NOT NULL REFERENCES tool_intents(intent_id),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    spec_revision INTEGER NOT NULL CHECK (spec_revision > 0),
    policy_revision INTEGER NOT NULL CHECK (policy_revision > 0),
    spec_hash TEXT NOT NULL,
    args_hash TEXT NOT NULL,
    tool_version INTEGER NOT NULL CHECK (tool_version = 1),
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'denied', 'revoked')),
    decision_source TEXT NOT NULL CHECK (decision_source IN ('task_policy', 'local_cli', 'pending')),
    expires_at INTEGER NOT NULL,
    uses_remaining INTEGER NOT NULL CHECK (uses_remaining IN (0, 1)),
    decided_at TEXT
) STRICT;
