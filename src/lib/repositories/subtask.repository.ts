import { getDb } from '$lib/db/client';
import type { Subtask } from '$lib/types/subtask';

const COLUMNS = `
	id,
	task_id AS taskId,
	title,
	completed,
	sort_order AS sortOrder,
	created_at AS createdAt
`;

export async function findByTaskId(taskId: string): Promise<Subtask[]> {
	try {
		const db = await getDb();
		return await db.select<Subtask[]>(
			`SELECT ${COLUMNS} FROM subtasks WHERE task_id = $1 ORDER BY sort_order ASC, created_at ASC`,
			[taskId]
		);
	} catch (error) {
		throw new Error('Failed to load subtasks', { cause: error });
	}
}

export async function create(taskId: string, title: string): Promise<Subtask> {
	try {
		const db = await getDb();
		const id = crypto.randomUUID();
		const now = new Date().toISOString();
		const countResult = await db.select<{ cnt: number }[]>(
			'SELECT COUNT(*) AS cnt FROM subtasks WHERE task_id = $1',
			[taskId]
		);
		const sortOrder = countResult[0]?.cnt ?? 0;
		const subtask: Subtask = { id, taskId, title, completed: false, sortOrder, createdAt: now };
		await db.execute(
			`INSERT INTO subtasks (id, task_id, title, completed, sort_order, created_at) VALUES ($1, $2, $3, 0, $4, $5)`,
			[id, taskId, title, sortOrder, now]
		);
		return subtask;
	} catch (error) {
		throw new Error('Failed to create subtask', { cause: error });
	}
}

export async function toggle(id: string, completed: boolean): Promise<void> {
	try {
		const db = await getDb();
		await db.execute('UPDATE subtasks SET completed = $1 WHERE id = $2', [completed ? 1 : 0, id]);
	} catch (error) {
		throw new Error('Failed to update subtask', { cause: error });
	}
}

export async function rename(id: string, title: string): Promise<void> {
	try {
		const db = await getDb();
		await db.execute('UPDATE subtasks SET title = $1 WHERE id = $2', [title, id]);
	} catch (error) {
		throw new Error('Failed to rename subtask', { cause: error });
	}
}

export async function remove(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const result = await db.execute('DELETE FROM subtasks WHERE id = $1', [id]);
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to delete subtask', { cause: error });
	}
}

export async function reorder(taskId: string, subtaskIds: string[]): Promise<void> {
	try {
		const db = await getDb();
		for (let i = 0; i < subtaskIds.length; i++) {
			await db.execute('UPDATE subtasks SET sort_order = $1 WHERE id = $2 AND task_id = $3', [
				i,
				subtaskIds[i],
				taskId
			]);
		}
	} catch (error) {
		throw new Error('Failed to reorder subtasks', { cause: error });
	}
}
