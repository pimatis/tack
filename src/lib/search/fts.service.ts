import { getDb } from '$lib/db/client';

// wrap terms as fts5 phrases so user input is matched as literal text
function toFtsQuery(query: string): string {
	return query
		.trim()
		.split(/\s+/)
		.map((term) => `"${term.replaceAll('"', '""')}"`)
		.join(' ');
}

// returns ids of tasks whose title, description or subtasks match the query
export async function searchTaskIds(query: string): Promise<Set<string>> {
	const db = await getDb();
	const rows = await db.select<{ task_id: string }[]>(
		'SELECT task_id FROM tasks_fts WHERE tasks_fts MATCH $1 LIMIT 500',
		[toFtsQuery(query)]
	);
	return new Set(rows.map((r) => r.task_id));
}
