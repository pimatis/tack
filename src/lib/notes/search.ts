import { getDb } from '$lib/db/client';
import { toFtsQuery } from '$lib/search/fts.service';

export type NoteSearchResult = { path: string; name: string; snippet: string };

// replace the whole index with the current folder contents
export async function reindexNotes(notes: { path: string; name: string; content: string }[]) {
	const db = await getDb();
	await db.execute('DELETE FROM notes_fts');
	for (const note of notes) {
		await db.execute('INSERT INTO notes_fts (path, name, content) VALUES ($1, $2, $3)', [
			note.path,
			note.name,
			note.content
		]);
	}
}

// update a single note row after an in-app save
export async function indexNote(path: string, name: string, content: string) {
	const db = await getDb();
	await db.execute('DELETE FROM notes_fts WHERE path = $1', [path]);
	await db.execute('INSERT INTO notes_fts (path, name, content) VALUES ($1, $2, $3)', [
		path,
		name,
		content
	]);
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
