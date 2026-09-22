-- note index tables. the app used to create these lazily while indexing, which
-- meant a client needed ddl rights on the live server; the schema lives in
-- migrations now so the live endpoint can refuse every ddl statement
CREATE TABLE IF NOT EXISTS notes_index_meta (
    path TEXT PRIMARY KEY,
    mtime INTEGER,
    size INTEGER
);

CREATE TABLE IF NOT EXISTS note_links (
    src TEXT,
    dst TEXT,
    kind TEXT
);
