CREATE TABLE instructions (
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    client_message_id TEXT NOT NULL CHECK (length(client_message_id) BETWEEN 1 AND 128),
    seq INTEGER NOT NULL CHECK (seq > 0),
    content TEXT NOT NULL CHECK (length(CAST(content AS BLOB)) BETWEEN 1 AND 8192),
    status TEXT NOT NULL DEFAULT 'received' CHECK (status IN ('received', 'applied')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (task_id, client_message_id),
    UNIQUE (task_id, seq)
) STRICT;
