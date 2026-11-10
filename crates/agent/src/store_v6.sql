CREATE TABLE task_runs (
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    generation INTEGER NOT NULL CHECK (generation > 0),
    recorded_state TEXT NOT NULL,
    last_seq INTEGER NOT NULL CHECK (last_seq > 0),
    artifact_created INTEGER CHECK (artifact_created IN (0, 1)),
    session_revision INTEGER,
    session_json TEXT,
    PRIMARY KEY (task_id, generation)
) STRICT;
