-- full text search over note name and markdown content
-- one fts row per note file, reindexed from the app on refresh and save
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(path UNINDEXED, name, content);
