import { getDb } from '$lib/db/client';
import type { ActivityLog, ActivityAction } from '$lib/types/activity';

const COLUMNS = `
	id,
	task_id AS taskId,
	action,
	field,
	old_value AS oldValue,
	new_value AS newValue,
	source,
	created_at AS createdAt
`;

export async function findByTaskId(taskId: string): Promise<ActivityLog[]> {
	try {
		const db = await getDb();
		return await db.select<ActivityLog[]>(
			`SELECT ${COLUMNS} FROM activity_log WHERE task_id = $1 ORDER BY created_at DESC LIMIT 50`,
			[taskId]
		);
	} catch (error) {
		throw new Error('Failed to load activity log', { cause: error });
	}
}

export async function log(
	taskId: string,
	action: ActivityAction,
	field?: string | null,
	oldValue?: string | null,
	newValue?: string | null
): Promise<void> {
	try {
		const db = await getDb();
		const id = crypto.randomUUID();
		const now = new Date().toISOString();
		await db.execute(
			`INSERT INTO activity_log (id, task_id, action, field, old_value, new_value, source, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
			[id, taskId, action, field ?? null, oldValue ?? null, newValue ?? null, 'gui', now]
		);
	} catch {
		// activity logging is non-critical, silently ignore
	}
}

export async function logBatch(
	taskId: string,
	entries: {
		action: ActivityAction;
		field?: string | null;
		oldValue?: string | null;
		newValue?: string | null;
	}[]
): Promise<void> {
	for (const entry of entries) {
		await log(taskId, entry.action, entry.field, entry.oldValue, entry.newValue);
	}
}
