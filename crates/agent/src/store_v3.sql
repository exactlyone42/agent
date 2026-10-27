CREATE TABLE events_v3 (
    event_id TEXT UNIQUE NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    schema_version INTEGER NOT NULL DEFAULT 3 CHECK (schema_version IN (1, 2, 3)),
    generation INTEGER NOT NULL DEFAULT 1 CHECK (generation > 0),
    seq INTEGER NOT NULL CHECK (seq > 0),
    event_type TEXT NOT NULL CHECK (event_type IN ('started', 'succeeded', 'needs_review', 'failed', 'cancelled', 'outcome_unknown', 'approval_requested', 'approval_granted', 'approval_denied', 'approval_expired', 'approval_revoked', 'tool_dispatched', 'tool_succeeded', 'tool_failed', 'tool_rejected', 'tool_outcome_unknown')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    related_id TEXT,
    PRIMARY KEY (task_id, seq)
) STRICT;
INSERT INTO events_v3 SELECT * FROM events;
DROP TABLE events;
ALTER TABLE events_v3 RENAME TO events;
CREATE TABLE tool_intents_v3 (
    intent_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    schema_version INTEGER NOT NULL DEFAULT 2 CHECK (schema_version IN (1, 2)),
    generation INTEGER NOT NULL CHECK (generation > 0),
    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL CHECK (tool_name IN ('read_input', 'write_output')),
    tool_version INTEGER NOT NULL CHECK (tool_version = 1),
    args_json TEXT NOT NULL,
    args_hash TEXT NOT NULL,
    scope_path TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('prepared', 'denied', 'dispatched', 'succeeded', 'failed', 'cancelled', 'outcome_unknown')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    step_id INTEGER NOT NULL CHECK (step_id >= 0),
    UNIQUE (task_id, generation, step_id, call_id)
) STRICT;
INSERT INTO tool_intents_v3 SELECT *, 0 FROM tool_intents;
CREATE TABLE approvals_v3 (
    approval_id TEXT PRIMARY KEY NOT NULL,
    intent_id TEXT UNIQUE NOT NULL REFERENCES tool_intents_v3(intent_id),
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version = 1),
    spec_revision INTEGER NOT NULL CHECK (spec_revision > 0),
    policy_revision INTEGER NOT NULL CHECK (policy_revision > 0),
    spec_hash TEXT NOT NULL,
    args_hash TEXT NOT NULL,
    tool_version INTEGER NOT NULL CHECK (tool_version = 1),
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'denied', 'revoked', 'expired')),
    decision_source TEXT NOT NULL CHECK (decision_source IN ('task_policy', 'local_cli', 'pending')),
    expires_at INTEGER NOT NULL,
    uses_remaining INTEGER NOT NULL CHECK (uses_remaining IN (0, 1)),
    decided_at TEXT
) STRICT;
INSERT INTO approvals_v3 SELECT * FROM approvals;
DROP TABLE approvals;
DROP TABLE tool_intents;
ALTER TABLE tool_intents_v3 RENAME TO tool_intents;
ALTER TABLE approvals_v3 RENAME TO approvals;
