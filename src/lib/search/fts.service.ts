import { getDb } from '$lib/db/client';

// wrap terms as fts5 phrases so user input is matched as literal text; with
// prefix, each term also matches longer tokens ("sad" finds "sadadsa")
export function toFtsQuery(query: string, prefix = false): string {
	return query
		.trim()
		.split(/\s+/)
		.map((term) => `"${term.replaceAll('"', '""')}"${prefix ? '*' : ''}`)
		.join(' ');
}

// returns ids of tasks matching the query: title, description, subtasks,
// label names, project name and the issue number (padded or bare)
export async function searchTaskIds(query: string): Promise<Set<string>> {
	const db = await getDb();
	const rows = await db.select<{ task_id: string }[]>(
		'SELECT task_id FROM tasks_fts WHERE tasks_fts MATCH $1 LIMIT 500',
		[toFtsQuery(query)]
	);
	return new Set(rows.map((r) => r.task_id));
}
