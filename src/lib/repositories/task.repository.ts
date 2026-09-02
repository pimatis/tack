import { getDb } from '$lib/db/client';
import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
import { statusConfig, priorityConfig } from '$lib/task/constants';
import {
	findAllTaskLabelIds,
	findLabelIdsByTaskId,
	setTaskLabels
} from '$lib/repositories/label.repository';
import {
	log as logActivity,
	logBatch as logActivityBatch
} from '$lib/repositories/activity.repository';

type CreateTaskInput = Pick<Task, 'title'> &
	Partial<
		Pick<Task, 'id' | 'description' | 'status' | 'priority' | 'projectId' | 'dueDate' | 'endDate'>
	>;

type UpdateTaskInput = Partial<
	Pick<Task, 'title' | 'description' | 'status' | 'priority' | 'dueDate' | 'endDate' | 'pinned'>
>;

// list columns: everything the list/board views render, minus the heavy description
const TASK_LIST_COLUMNS = `
	id,
	number,
	project_id AS projectId,
	title,
	status,
	priority,
	due_date AS dueDate,
	end_date AS endDate,
	sort_order AS sortOrder,
	pinned,
	created_at AS createdAt,
	updated_at AS updatedAt,
	deleted_at AS deletedAt
`;

// full columns: list columns + description, for detail views
const TASK_COLUMNS = `${TASK_LIST_COLUMNS},
	description
`;

export async function create(input: CreateTaskInput): Promise<Task> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const id = input.id ?? crypto.randomUUID();
		const projectId = input.projectId ?? null;

		// assign next sequential number per project
		const numResult = projectId
			? await db.select<{ max_num: number }[]>(
					'SELECT COALESCE(MAX(number), 0) AS max_num FROM tasks WHERE project_id = $1',
					[projectId]
				)
			: await db.select<{ max_num: number }[]>(
					'SELECT COALESCE(MAX(number), 0) AS max_num FROM tasks WHERE project_id IS NULL'
				);
		const number = (numResult[0]?.max_num ?? 0) + 1;

		const task: Task = {
			id,
			number,
			projectId,
			title: input.title,
			description: input.description ?? null,
			status: input.status ?? 'todo',
			priority: input.priority ?? 0,
			dueDate: input.dueDate ?? null,
			endDate: input.endDate ?? null,
			sortOrder: 0,
			createdAt: now,
			updatedAt: now,
			labelIds: []
		};

		await db.execute(
			`INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, sort_order, created_at, updated_at)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, $10, $11)`,
			[
				task.id,
				task.number,
				task.projectId ?? null,
				task.title,
				task.description,
				task.status,
				task.priority,
				task.dueDate,
				task.endDate,
				task.createdAt,
				task.updatedAt
			]
		);

		void logActivity(id, 'created');

		return task;
	} catch (error) {
		throw new Error('Failed to create task', { cause: error });
	}
}

export async function findAll(): Promise<Task[]> {
	try {
		const db = await getDb();
		const tasks = await db.select<Task[]>(
			`SELECT ${TASK_LIST_COLUMNS} FROM tasks WHERE deleted_at IS NULL ORDER BY updated_at DESC`
		);
		const labelMap = await findAllTaskLabelIds();
		for (const task of tasks) {
			task.labelIds = labelMap.get(task.id) ?? [];
			task.pinned = Boolean(task.pinned);
		}
		return tasks;
	} catch (error) {
		throw new Error('Failed to load tasks', { cause: error });
	}
}

async function findById(id: string): Promise<Task | null> {
	try {
		const db = await getDb();
		const tasks = await db.select<Task[]>(`SELECT ${TASK_COLUMNS} FROM tasks WHERE id = $1`, [id]);
		if (tasks.length === 0) return null;
		tasks[0].labelIds = await findLabelIdsByTaskId(id);
		tasks[0].pinned = Boolean(tasks[0].pinned);
		return tasks[0];
	} catch (error) {
		throw new Error('Failed to load task', { cause: error });
	}
}

export async function togglePin(id: string): Promise<Task | null> {
	try {
		const current = await findById(id);
		if (!current) return null;
		return await update(id, { pinned: !current.pinned });
	} catch (error) {
		throw new Error('Failed to toggle pin', { cause: error });
	}
}

export async function update(id: string, input: UpdateTaskInput): Promise<Task | null> {
	try {
		const current = await findById(id);
		if (!current) return null;

		const assignments: string[] = [];
		const values: unknown[] = [];
		const activityEntries: {
			action: string;
			field?: string | null;
			oldValue?: string | null;
			newValue?: string | null;
		}[] = [];

		for (const [column, value, field, action] of [
			['title', input.title, 'title', 'title_changed'],
			['description', input.description, 'description', 'description_changed'],
			['status', input.status, 'status', 'status_changed'],
			['priority', input.priority, 'priority', 'priority_changed'],
			['due_date', input.dueDate, 'due_date', 'due_date_changed'],
			['end_date', input.endDate, 'end_date', 'end_date_changed'],
			[
				'pinned',
				input.pinned !== undefined ? (input.pinned ? 1 : 0) : undefined,
				'pinned',
				'pinned_changed'
			]
		] as const) {
			if (value === undefined) continue;

			const oldValue = (current as Record<string, unknown>)[field as string];
			if (oldValue === value) continue;

			values.push(value);
			assignments.push(`${column} = $${values.length}`);

			// format readable values for activity log
			const oldDisplay =
				field === 'status'
					? (statusConfig[String(oldValue) as TaskStatus]?.label ?? String(oldValue))
					: field === 'priority'
						? (priorityConfig[Number(oldValue) as TaskPriority]?.label ?? String(oldValue))
						: String(oldValue ?? '');
			const newDisplay =
				field === 'status'
					? (statusConfig[String(value) as TaskStatus]?.label ?? String(value))
					: field === 'priority'
						? (priorityConfig[Number(value) as TaskPriority]?.label ?? String(value))
						: String(value ?? '');

			activityEntries.push({
				action: action as string,
				field: field as string,
				oldValue: oldDisplay || null,
				newValue: newDisplay || null
			});
		}

		if (assignments.length === 0) return current;

		values.push(new Date().toISOString());
		assignments.push(`updated_at = $${values.length}`);
		values.push(id);

		const db = await getDb();
		const result = await db.execute(
			`UPDATE tasks SET ${assignments.join(', ')} WHERE id = $${values.length}`,
			values
		);

		if (result.rowsAffected === 0) return null;

		// log activity entries
		void logActivityBatch(
			id,
			activityEntries.map((e) => ({
				action: e.action as Parameters<typeof logActivity>[1],
				field: e.field,
				oldValue: e.oldValue,
				newValue: e.newValue
			}))
		);

		return await findById(id);
	} catch (error) {
		throw new Error('Failed to update task', { cause: error });
	}
}

export async function reorderInStatus(status: TaskStatus, taskIds: string[]): Promise<void> {
	try {
		const db = await getDb();
		for (let i = 0; i < taskIds.length; i++) {
			await db.execute('UPDATE tasks SET sort_order = $1 WHERE id = $2', [i, taskIds[i]]);
		}
	} catch (error) {
		throw new Error('Failed to reorder tasks', { cause: error });
	}
}

export async function remove(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const result = await db.execute(
			'UPDATE tasks SET deleted_at = $1, updated_at = $2 WHERE id = $3 AND deleted_at IS NULL',
			[now, now, id]
		);
		if (result.rowsAffected > 0) {
			void logActivity(id, 'trashed');
		}
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to delete task', { cause: error });
	}
}

export async function bulkDelete(ids: string[]): Promise<void> {
	if (ids.length === 0) return;
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const placeholders = ids.map((_, i) => `$${i + 3}`).join(', ');
		await db.execute(
			`UPDATE tasks SET deleted_at = $1, updated_at = $2 WHERE id IN (${placeholders}) AND deleted_at IS NULL`,
			[now, now, ...ids]
		);
		for (const id of ids) {
			void logActivity(id, 'trashed');
		}
	} catch (error) {
		throw new Error('Failed to delete tasks', { cause: error });
	}
}

export async function bulkUpdateStatus(ids: string[], status: TaskStatus): Promise<void> {
	if (ids.length === 0) return;
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const placeholders = ids.map((_, i) => `$${i + 3}`).join(', ');
		await db.execute(
			`UPDATE tasks SET status = $1, updated_at = $2 WHERE id IN (${placeholders})`,
			[status, now, ...ids]
		);
		for (const id of ids) {
			void logActivity(id, 'status_changed', 'status', undefined, statusConfig[status].label);
		}
	} catch (error) {
		throw new Error('Failed to update status', { cause: error });
	}
}

export async function bulkUpdatePriority(ids: string[], priority: TaskPriority): Promise<void> {
	if (ids.length === 0) return;
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const placeholders = ids.map((_, i) => `$${i + 3}`).join(', ');
		await db.execute(
			`UPDATE tasks SET priority = $1, updated_at = $2 WHERE id IN (${placeholders})`,
			[priority, now, ...ids]
		);
		for (const id of ids) {
			void logActivity(
				id,
				'priority_changed',
				'priority',
				undefined,
				priorityConfig[priority].label
			);
		}
	} catch (error) {
		throw new Error('Failed to update priority', { cause: error });
	}
}

export async function bulkUpdateProject(ids: string[], projectId: string | null): Promise<void> {
	if (ids.length === 0) return;
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const placeholders = ids.map((_, i) => `$${i + 3}`).join(', ');
		await db.execute(
			`UPDATE tasks SET project_id = $1, updated_at = $2 WHERE id IN (${placeholders})`,
			[projectId, now, ...ids]
		);
	} catch (error) {
		throw new Error('Failed to update project', { cause: error });
	}
}

export async function duplicate(id: string): Promise<Task | null> {
	try {
		const original = await findById(id);
		if (!original) return null;
		const copy = await create({
			title: `${original.title} (copy)`,
			description: original.description,
			status: original.status,
			priority: original.priority,
			projectId: original.projectId,
			dueDate: original.dueDate,
			endDate: original.endDate
		});
		if (original.labelIds && original.labelIds.length > 0) {
			await setTaskLabels(copy.id, original.labelIds);
			copy.labelIds = [...original.labelIds];
		}
		return copy;
	} catch (error) {
		throw new Error('Failed to duplicate task', { cause: error });
	}
}

export async function findTrashed(): Promise<Task[]> {
	try {
		const db = await getDb();
		const tasks = await db.select<Task[]>(
			`SELECT ${TASK_LIST_COLUMNS} FROM tasks WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC`
		);
		const labelMap = await findAllTaskLabelIds();
		for (const task of tasks) {
			task.labelIds = labelMap.get(task.id) ?? [];
			task.pinned = Boolean(task.pinned);
		}
		return tasks;
	} catch (error) {
		throw new Error('Failed to load trashed tasks', { cause: error });
	}
}

export async function restore(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const result = await db.execute(
			'UPDATE tasks SET deleted_at = NULL, updated_at = $1 WHERE id = $2 AND deleted_at IS NOT NULL',
			[now, id]
		);
		if (result.rowsAffected > 0) {
			void logActivity(id, 'restored');
		}
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to restore task', { cause: error });
	}
}

export async function permanentDelete(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const result = await db.execute('DELETE FROM tasks WHERE id = $1 AND deleted_at IS NOT NULL', [
			id
		]);
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to permanently delete task', { cause: error });
	}
}

export async function emptyTrash(): Promise<void> {
	try {
		const db = await getDb();
		await db.execute('DELETE FROM tasks WHERE deleted_at IS NOT NULL');
	} catch (error) {
		throw new Error('Failed to empty trash', { cause: error });
	}
}
