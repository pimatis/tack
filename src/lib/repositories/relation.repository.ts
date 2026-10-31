import { getDb } from '$lib/db/client';
import { log as logActivity } from '$lib/repositories/activity.repository';
import { toFtsQuery } from '$lib/search/fts.service';
import type { RelationType, TaskRelation } from '$lib/types/relation';
import type { Task } from '$lib/types/task';

// both directions in one query: `out` rows are relations this task declared,
// `in` rows are relations other tasks declared against it
export async function findForTask(taskId: string): Promise<TaskRelation[]> {
	try {
		const db = await getDb();
		return await db.select<TaskRelation[]>(
			`SELECT r.task_id AS taskId, r.related_task_id AS relatedTaskId, r.type,
				'out' AS direction,
				o.id AS otherId, o.number AS otherNumber, o.title AS otherTitle,
				o.status AS otherStatus, p.prefix AS otherPrefix
			 FROM task_relations r
			 JOIN tasks o ON o.id = r.related_task_id
			 LEFT JOIN projects p ON p.id = o.project_id
			 WHERE r.task_id = $1 AND o.deleted_at IS NULL
			 UNION ALL
			 SELECT r.task_id, r.related_task_id, r.type,
				'in' AS direction,
				o.id, o.number, o.title, o.status, p.prefix
			 FROM task_relations r
			 JOIN tasks o ON o.id = r.task_id
			 LEFT JOIN projects p ON p.id = o.project_id
			 WHERE r.related_task_id = $1 AND o.deleted_at IS NULL`,
			[taskId]
		);
	} catch (error) {
		throw new Error('Failed to load relations', { cause: error });
	}
}

export async function add(taskId: string, relatedTaskId: string, type: RelationType) {
	if (taskId === relatedTaskId) throw new Error('A task cannot relate to itself');
	try {
		const db = await getDb();
		await db.execute(
			`INSERT OR IGNORE INTO task_relations (task_id, related_task_id, type, created_at)
			 VALUES ($1, $2, $3, $4)`,
			[taskId, relatedTaskId, type, new Date().toISOString()]
		);
		void logActivity(taskId, 'relation_added', 'relation', null, `${type}:${relatedTaskId}`);
	} catch (error) {
		throw new Error('Failed to add relation', { cause: error });
	}
}

export async function remove(taskId: string, relatedTaskId: string, type: RelationType) {
	try {
		const db = await getDb();
		// the row may have been created in either direction; delete both sides
		await db.execute(
			`DELETE FROM task_relations
			 WHERE type = $3 AND ((task_id = $1 AND related_task_id = $2) OR (task_id = $2 AND related_task_id = $1))`,
			[taskId, relatedTaskId, type]
		);
		void logActivity(taskId, 'relation_removed', 'relation', `${type}:${relatedTaskId}`, null);
	} catch (error) {
		throw new Error('Failed to remove relation', { cause: error });
	}
}

// lightweight task rows for the children list; full Task objects are fetched
// on demand when one is opened
export type ChildTask = Pick<Task, 'id' | 'number' | 'title' | 'status'> & {
	prefix: string | null;
};

export async function findChildren(parentId: string): Promise<ChildTask[]> {
	try {
		const db = await getDb();
		return await db.select<ChildTask[]>(
			`SELECT t.id, t.number, t.title, t.status, p.prefix
			 FROM tasks t
			 LEFT JOIN projects p ON p.id = t.project_id
			 WHERE t.parent_id = $1 AND t.deleted_at IS NULL
			 ORDER BY t.sort_order, t.created_at`,
			[parentId]
		);
	} catch (error) {
		throw new Error('Failed to load child tasks', { cause: error });
	}
}

// candidates for relation/parent pickers: recent tasks or an fts match,
// excluding the task itself
export async function searchTaskCandidates(query: string, excludeId: string): Promise<ChildTask[]> {
	try {
		const db = await getDb();
		const q = query.trim();
		if (!q) {
			return await db.select<ChildTask[]>(
				`SELECT t.id, t.number, t.title, t.status, p.prefix
				 FROM tasks t
				 LEFT JOIN projects p ON p.id = t.project_id
				 WHERE t.deleted_at IS NULL AND t.id != $1
				 ORDER BY t.updated_at DESC LIMIT 12`,
				[excludeId]
			);
		}
		return await db.select<ChildTask[]>(
			`SELECT t.id, t.number, t.title, t.status, p.prefix
			 FROM tasks_fts
			 JOIN tasks t ON t.id = tasks_fts.task_id
			 LEFT JOIN projects p ON p.id = t.project_id
			 WHERE tasks_fts MATCH $1 AND t.deleted_at IS NULL AND t.id != $2
			 ORDER BY rank LIMIT 12`,
			[toFtsQuery(q), excludeId]
		);
	} catch (error) {
		throw new Error('Failed to search tasks', { cause: error });
	}
}
