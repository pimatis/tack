CREATE TABLE agents (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    harness TEXT,
    color TEXT NOT NULL DEFAULT 'indigo',
    created_at TEXT NOT NULL,
    last_seen_at TEXT
);
