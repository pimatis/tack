import { getDb } from '$lib/db/client';

export type TaskNoteLink = {
	notePath: string;
	createdAt: string;
};

export async function findByTaskId(taskId: string): Promise<TaskNoteLink[]> {
	try {
		const db = await getDb();
		return await db.select<TaskNoteLink[]>(
			'SELECT note_path AS notePath, created_at AS createdAt FROM task_notes WHERE task_id = $1 ORDER BY created_at ASC',
			[taskId]
		);
	} catch (error) {
		throw new Error('Failed to load linked notes', { cause: error });
	}
}

// tasks explicitly linked to a note from the task panel, for the note side
export async function findByNotePath(notePath: string): Promise<{ id: string; title: string }[]> {
	try {
		const db = await getDb();
		return await db.select<{ id: string; title: string }[]>(
			`SELECT t.id AS id, t.title AS title FROM task_notes tn
			 JOIN tasks t ON t.id = tn.task_id
			 WHERE tn.note_path = $1 AND t.deleted_at IS NULL
			 ORDER BY tn.created_at ASC`,
			[notePath]
		);
	} catch (error) {
		throw new Error('Failed to load linked tasks', { cause: error });
	}
}

export async function add(taskId: string, notePath: string): Promise<void> {
	try {
		const db = await getDb();
		await db.execute(
			`INSERT INTO task_notes (task_id, note_path, created_at) VALUES ($1, $2, $3)
			 ON CONFLICT(task_id, note_path) DO NOTHING`,
			[taskId, notePath, new Date().toISOString()]
		);
	} catch (error) {
		throw new Error('Failed to link note', { cause: error });
	}
}

export async function remove(taskId: string, notePath: string): Promise<void> {
	try {
		const db = await getDb();
		await db.execute('DELETE FROM task_notes WHERE task_id = $1 AND note_path = $2', [
			taskId,
			notePath
		]);
	} catch (error) {
		throw new Error('Failed to unlink note', { cause: error });
	}
}

// rename/move support: keep explicit links pointing at the note's new path
export async function remapPaths(remap: Map<string, string>): Promise<void> {
	if (remap.size === 0) return;
	const db = await getDb();
	for (const [oldPath, newPath] of remap) {
		await db.execute('UPDATE task_notes SET note_path = $1 WHERE note_path = $2', [
			newPath,
			oldPath
		]);
	}
}
