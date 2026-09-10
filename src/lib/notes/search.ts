import { getDb, type DbClient } from '$lib/db/client';
import { toFtsQuery } from '$lib/search/fts.service';
import { extractLinks } from './links';

export type NoteSearchResult = { path: string; name: string; snippet: string };

export type IndexedNote = {
	path: string;
	name: string;
	content: string;
	modified: number;
	size: number;
};

async function ensureTables(db: DbClient) {
	await db.execute(
		'CREATE TABLE IF NOT EXISTS notes_index_meta (path TEXT PRIMARY KEY, mtime INTEGER, size INTEGER)'
	);
	await db.execute('CREATE TABLE IF NOT EXISTS note_links (src TEXT, dst TEXT, kind TEXT)');
}

// incremental index rebuild: only notes whose mtime or size changed are
// re-parsed, so large vaults stay cheap on every refresh
export async function reindexNotes(notes: IndexedNote[]) {
	const db = await getDb();
	await ensureTables(db);
	const meta = await db.select<{ path: string; mtime: number; size: number }[]>(
		'SELECT path, mtime, size FROM notes_index_meta'
	);
	const known = new Map(meta.map((m) => [m.path, m]));
	const seen = new Set<string>();
	for (const note of notes) {
		seen.add(note.path);
		const prev = known.get(note.path);
		if (prev && prev.mtime === note.modified && prev.size === note.size) continue;
		await db.execute('DELETE FROM notes_fts WHERE path = $1', [note.path]);
		await db.execute('INSERT INTO notes_fts (path, name, content) VALUES ($1, $2, $3)', [
			note.path,
			note.name,
			note.content
		]);
		await db.execute(
			'INSERT INTO notes_index_meta (path, mtime, size) VALUES ($1, $2, $3) ON CONFLICT(path) DO UPDATE SET mtime = $2, size = $3',
			[note.path, note.modified, note.size]
		);
		await indexLinks(db, note.path, note.content);
	}
	// rows for notes that no longer exist on disk
	for (const stale of known.keys()) {
		if (seen.has(stale)) continue;
		await db.execute('DELETE FROM notes_fts WHERE path = $1', [stale]);
		await db.execute('DELETE FROM notes_index_meta WHERE path = $1', [stale]);
		await db.execute('DELETE FROM note_links WHERE src = $1', [stale]);
	}
}

// store every outgoing link of one note; wiki links are stored under their
// name (wiki:Name) and resolved against note names when backlinks are queried
async function indexLinks(db: DbClient, src: string, content: string) {
	await db.execute('DELETE FROM note_links WHERE src = $1', [src]);
	const links = extractLinks(content);
	for (const dst of links.noteTargets) {
		await db.execute('INSERT INTO note_links (src, dst, kind) VALUES ($1, $2, $3)', [
			src,
			dst,
			'note'
		]);
	}
	for (const id of links.taskIds) {
		await db.execute('INSERT INTO note_links (src, dst, kind) VALUES ($1, $2, $3)', [
			src,
			`task:${id}`,
			'task'
		]);
	}
	for (const name of links.wikiNames) {
		await db.execute('INSERT INTO note_links (src, dst, kind) VALUES ($1, $2, $3)', [
			src,
			`wiki:${name}`,
			'wiki'
		]);
	}
}

// update a single note row after an in-app save
export async function indexNote(
	path: string,
	name: string,
	content: string,
	modified = Date.now()
) {
	const db = await getDb();
	await ensureTables(db);
	await db.execute('DELETE FROM notes_fts WHERE path = $1', [path]);
	await db.execute('INSERT INTO notes_fts (path, name, content) VALUES ($1, $2, $3)', [
		path,
		name,
		content
	]);
	await db.execute(
		'INSERT INTO notes_index_meta (path, mtime, size) VALUES ($1, $2, $3) ON CONFLICT(path) DO UPDATE SET mtime = $2, size = $3',
		[path, modified, content.length]
	);
	await indexLinks(db, path, content);
}

// notes whose title or content matches the query, with a content snippet
export async function searchNotes(query: string): Promise<NoteSearchResult[]> {
	if (!query.trim()) return [];
	const db = await getDb();
	return db.select<NoteSearchResult[]>(
		`SELECT path, name, snippet(notes_fts, 2, '', '', '…', 8) AS snippet
		 FROM notes_fts WHERE notes_fts MATCH $1 ORDER BY rank LIMIT 50`,
		[toFtsQuery(query)]
	);
}

// notes linking to the given target through the links index
export async function getLinkingNotes(target: string): Promise<{ path: string; name: string }[]> {
	const db = await getDb();
	return db.select<{ path: string; name: string }[]>(
		`SELECT f.path, f.name FROM note_links l
		 JOIN notes_fts f ON f.path = l.src
		 WHERE l.dst = $1 ORDER BY f.name`,
		[target]
	);
}
