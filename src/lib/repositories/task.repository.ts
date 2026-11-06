import { getDb } from '$lib/db/client';
import { newId } from '$lib/utils';
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
import {
	nextOccurrence,
	addDaysIso,
	addDaysTimestamp,
	recurrenceLabel
} from '$lib/task/recurrence';

type CreateTaskInput = Pick<Task, 'title'> &
	Partial<
		Pick<
			Task,
			| 'id'
			| 'description'
			| 'status'
			| 'priority'
			| 'projectId'
			| 'dueDate'
			| 'endDate'
			| 'reminderAt'
			| 'recurrenceRule'
			| 'recurrenceParentId'
			| 'parentId'
			| 'estimateMinutes'
		>
	>;

type UpdateTaskInput = Partial<
	Pick<
		Task,
		| 'title'
		| 'description'
		| 'status'
		| 'priority'
		| 'dueDate'
		| 'endDate'
		| 'pinned'
		| 'reminderAt'
		| 'recurrenceRule'
		| 'parentId'
		| 'estimateMinutes'
		| 'assignee'
	>
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
	reminder_at AS reminderAt,
	reminder_sent_at AS reminderSentAt,
	recurrence_rule AS recurrenceRule,
	recurrence_parent_id AS recurrenceParentId,
	parent_id AS parentId,
	estimate_minutes AS estimateMinutes,
	tracked_seconds AS trackedSeconds,
	timer_started_at AS timerStartedAt,
	sort_order AS sortOrder,
	pinned,
	assignee,
	created_at AS createdAt,
	updated_at AS updatedAt,
	deleted_at AS deletedAt,
	EXISTS (
		SELECT 1 FROM task_relations r
		JOIN tasks b ON b.id = r.task_id
		WHERE r.related_task_id = tasks.id AND r.type = 'blocks'
			AND b.status NOT IN ('done', 'canceled') AND b.deleted_at IS NULL
	) AS blocked,
	(SELECT COUNT(*) FROM tasks c WHERE c.parent_id = tasks.id AND c.deleted_at IS NULL) AS childrenTotal,
	(SELECT COUNT(*) FROM tasks c WHERE c.parent_id = tasks.id AND c.deleted_at IS NULL AND c.status = 'done') AS childrenDone
`;

// full columns: list columns + description, for detail views
const TASK_COLUMNS = `${TASK_LIST_COLUMNS},
	description
`;

export async function create(input: CreateTaskInput): Promise<Task> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const id = input.id ?? newId();
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
			reminderAt: input.reminderAt ?? null,
			recurrenceRule: input.recurrenceRule ?? null,
			recurrenceParentId: input.recurrenceParentId ?? null,
			parentId: input.parentId ?? null,
			estimateMinutes: input.estimateMinutes ?? null,
			trackedSeconds: 0,
			timerStartedAt: null,
			sortOrder: 0,
			createdAt: now,
			updatedAt: now,
			labelIds: []
		};

		await db.execute(
			`INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, end_date, reminder_at, recurrence_rule, recurrence_parent_id, parent_id, estimate_minutes, sort_order, created_at, updated_at)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 0, $15, $16)`,
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
				task.reminderAt,
				task.recurrenceRule,
				task.recurrenceParentId,
				task.parentId,
				task.estimateMinutes,
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

export type Agent = {
	name: string;
	harness?: string | null;
	color?: string;
	lastSeenAt?: string | null;
};

// registered agents from the agents table, populated lazily on assign/activity
export async function findAgents(): Promise<Agent[]> {
	try {
		const db = await getDb();
		return await db.select<Agent[]>(
			`SELECT name, harness, color, last_seen_at AS lastSeenAt FROM agents ORDER BY name`
		);
	} catch {
		return [];
	}
}

// every assignee string seen on live tasks, for pickers and badges
export async function findAssignees(): Promise<string[]> {
	try {
		const db = await getDb();
		const rows = await db.select<{ assignee: string }[]>(
			`SELECT DISTINCT assignee FROM tasks WHERE assignee IS NOT NULL AND deleted_at IS NULL ORDER BY assignee`
		);
		return rows.map((r) => r.assignee);
	} catch {
		return [];
	}
}

// tasks currently in progress with an owner; the agent settings tab lists
// them as "who is working on what right now"
export async function findWorkingTasks(): Promise<Task[]> {
	try {
		const db = await getDb();
		return await db.select<Task[]>(
			`SELECT ${TASK_LIST_COLUMNS} FROM tasks
			 WHERE deleted_at IS NULL AND status = 'in_progress' AND assignee IS NOT NULL
			 ORDER BY assignee, updated_at DESC LIMIT 20`
		);
	} catch (error) {
		throw new Error('Failed to load working tasks', { cause: error });
	}
}

export async function findById(id: string): Promise<Task | null> {
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

		// reminders are a moment in time, so they compare against the task's
		// own field instead of the column-name loop above
		if (input.reminderAt !== undefined && (current.reminderAt ?? null) !== input.reminderAt) {
			values.push(input.reminderAt);
			assignments.push(`reminder_at = $${values.length}`);
			// a new reminder time must be able to notify again
			assignments.push('reminder_sent_at = NULL');
			activityEntries.push({
				action: 'reminder_changed',
				field: 'reminder_at',
				oldValue: current.reminderAt ? new Date(current.reminderAt).toLocaleString() : null,
				newValue: input.reminderAt ? new Date(input.reminderAt).toLocaleString() : null
			});
		}

		// the rule is stored as a compact string, so the activity log gets the
		// readable schedule instead
		if (
			input.recurrenceRule !== undefined &&
			(current.recurrenceRule ?? null) !== input.recurrenceRule
		) {
			values.push(input.recurrenceRule);
			assignments.push(`recurrence_rule = $${values.length}`);
			activityEntries.push({
				action: 'recurrence_changed',
				field: 'recurrence_rule',
				oldValue: recurrenceLabel(current.recurrenceRule),
				newValue: recurrenceLabel(input.recurrenceRule)
			});
		}

		if (
			input.estimateMinutes !== undefined &&
			(current.estimateMinutes ?? null) !== input.estimateMinutes
		) {
			values.push(input.estimateMinutes);
			assignments.push(`estimate_minutes = $${values.length}`);
			activityEntries.push({
				action: 'estimate_changed',
				field: 'estimate_minutes',
				oldValue: current.estimateMinutes != null ? `${current.estimateMinutes}m` : null,
				newValue: input.estimateMinutes != null ? `${input.estimateMinutes}m` : null
			});
		}

		if (input.assignee !== undefined && (current.assignee ?? null) !== input.assignee) {
			values.push(input.assignee);
			assignments.push(`assignee = $${values.length}`);
			activityEntries.push({
				action: input.assignee ? 'assigned' : 'unassigned',
				field: 'assignee',
				oldValue: current.assignee ?? null,
				newValue: input.assignee ?? null
			});
		}

		if (input.parentId !== undefined && (current.parentId ?? null) !== input.parentId) {
			// a cycle would orphan the whole subtree in any tree walk
			if (input.parentId && (await isAncestorOf(id, input.parentId))) {
				throw new Error('Cannot set parent: it would create a cycle');
			}
			values.push(input.parentId);
			assignments.push(`parent_id = $${values.length}`);
			activityEntries.push({
				action: 'parent_changed',
				field: 'parent_id',
				oldValue: current.parentId ?? null,
				newValue: input.parentId ?? null
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
		if (input.status === 'done' || input.status === 'canceled') {
			await bankTerminalTimers([id]);
		}

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

		const updated = await findById(id);

		// completing a recurring task rolls the series forward: this row stays
		// as history and a fresh todo takes the next date
		if (updated && current.status !== 'done' && updated.status === 'done') {
			await spawnNextOccurrence(updated);
		}

		return updated;
	} catch (error) {
		throw new Error('Failed to update task', { cause: error });
	}
}

// walk the ancestor chain of candidateParent; true when taskId is in it, which
// would make "taskId's parent = candidateParent" a cycle
async function isAncestorOf(taskId: string, candidateParent: string): Promise<boolean> {
	const db = await getDb();
	let current: string | null = candidateParent;
	for (let i = 0; i < 100; i++) {
		if (current === taskId) return true;
		const rows: { parent_id: string | null }[] = await db.select(
			'SELECT parent_id FROM tasks WHERE id = $1',
			[current]
		);
		current = rows[0]?.parent_id ?? null;
		if (!current) return false;
	}
	return true;
}

// copy a completed occurrence forward on its schedule; the recurrence parent
// keeps the series linked and stops the same date from spawning twice when a
// task is checked done again
async function spawnNextOccurrence(parent: Task): Promise<Task | null> {
	const next = nextOccurrence(parent.recurrenceRule, parent.dueDate);
	if (!next) return null;

	const db = await getDb();
	const existing = await db.select<{ id: string }[]>(
		`SELECT id FROM tasks WHERE recurrence_parent_id = $1 AND due_date = $2 AND deleted_at IS NULL`,
		[parent.id, next.dueDate]
	);
	if (existing.length > 0) return null;

	const occurrence = await create({
		title: parent.title,
		description: parent.description,
		projectId: parent.projectId ?? null,
		priority: parent.priority,
		dueDate: next.dueDate,
		endDate: parent.endDate ? addDaysIso(parent.endDate, next.deltaDays) : null,
		reminderAt: parent.reminderAt ? addDaysTimestamp(parent.reminderAt, next.deltaDays) : null,
		recurrenceRule: parent.recurrenceRule,
		recurrenceParentId: parent.id
	});

	const labelIds = parent.labelIds ?? (await findLabelIdsByTaskId(parent.id));
	if (labelIds.length > 0) {
		await setTaskLabels(occurrence.id, labelIds);
		occurrence.labelIds = [...labelIds];
	}

	void logActivity(
		parent.id,
		'recurrence_spawned',
		'due_date',
		parent.dueDate ?? null,
		next.dueDate
	);
	return occurrence;
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

// a terminal status must not leave a timer running: bank the elapsed segment
// and clear the start marker. runs after the status write so the stored
// status is the source of truth
async function bankTerminalTimers(ids: string[]): Promise<void> {
	const db = await getDb();
	const now = new Date().toISOString();
	const placeholders = ids.map((_, i) => `$${i + 2}`).join(', ');
	await db.execute(
		`UPDATE tasks SET
			tracked_seconds = tracked_seconds + MAX(0, CAST((julianday($1) - julianday(timer_started_at)) * 86400 AS INTEGER)),
			timer_started_at = NULL
		 WHERE id IN (${placeholders}) AND timer_started_at IS NOT NULL AND status IN ('done', 'canceled')`,
		[now, ...ids]
	);
}

export async function bulkUpdateStatus(ids: string[], status: TaskStatus): Promise<void> {
	if (ids.length === 0) return;
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		const placeholders = ids.map((_, i) => `$${i + 3}`).join(', ');
		// completing recurring tasks rolls each series forward; read them
		// before the update clears their old status
		const recurring =
			status === 'done'
				? await db.select<Task[]>(
						`SELECT ${TASK_COLUMNS} FROM tasks
						 WHERE id IN (${ids.map((_, i) => `$${i + 1}`).join(', ')})
						   AND status != 'done'
						   AND recurrence_rule IS NOT NULL
						   AND deleted_at IS NULL`,
						ids
					)
				: [];
		await db.execute(
			`UPDATE tasks SET status = $1, updated_at = $2 WHERE id IN (${placeholders})`,
			[status, now, ...ids]
		);
		if (status === 'done' || status === 'canceled') {
			await bankTerminalTimers(ids);
		}
		for (const id of ids) {
			void logActivity(id, 'status_changed', 'status', undefined, statusConfig[status].label);
		}
		for (const task of recurring) {
			await spawnNextOccurrence(task);
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
			endDate: original.endDate,
			reminderAt: original.reminderAt,
			recurrenceRule: original.recurrenceRule
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

// ---- time tracking ----

// one running timer at a time: starting banks any other timer's elapsed first
export async function startTimer(id: string): Promise<void> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		await db.execute(
			`UPDATE tasks SET
				tracked_seconds = tracked_seconds + MAX(0, CAST((julianday($1) - julianday(timer_started_at)) * 86400 AS INTEGER)),
				timer_started_at = NULL,
				updated_at = $1
			 WHERE timer_started_at IS NOT NULL`,
			[now]
		);
		await db.execute('UPDATE tasks SET timer_started_at = $1, updated_at = $1 WHERE id = $2', [
			now,
			id
		]);
		void logActivity(id, 'timer_started');
	} catch (error) {
		throw new Error('Failed to start timer', { cause: error });
	}
}

export async function stopTimer(id: string): Promise<void> {
	try {
		const db = await getDb();
		const now = new Date().toISOString();
		await db.execute(
			`UPDATE tasks SET
				tracked_seconds = tracked_seconds + MAX(0, CAST((julianday($1) - julianday(timer_started_at)) * 86400 AS INTEGER)),
				timer_started_at = NULL,
				updated_at = $1
			 WHERE id = $2 AND timer_started_at IS NOT NULL`,
			[now, id]
		);
		void logActivity(id, 'timer_stopped');
	} catch (error) {
		throw new Error('Failed to stop timer', { cause: error });
	}
}

// ---- reminders ----

export type DueReminder = {
	id: string;
	number: number;
	title: string;
	prefix: string | null;
	reminderAt: string;
	dueDate: string | null;
};

// reminders that have come due and were not notified yet; done/canceled and
// trashed tasks are skipped
export async function findDueReminders(nowIso: string): Promise<DueReminder[]> {
	const db = await getDb();
	return await db.select<DueReminder[]>(
		`SELECT t.id, t.number, t.title, p.prefix AS prefix, t.reminder_at AS reminderAt, t.due_date AS dueDate
		 FROM tasks t
		 LEFT JOIN projects p ON t.project_id = p.id
		 WHERE t.reminder_at IS NOT NULL
		   AND t.reminder_sent_at IS NULL
		   AND t.deleted_at IS NULL
		   AND t.status NOT IN ('done', 'canceled')
		   AND t.reminder_at <= $1
		 ORDER BY t.reminder_at ASC`,
		[nowIso]
	);
}

// mark a reminder as notified so it never fires twice
export async function markReminderSent(id: string, sentAt: string): Promise<void> {
	const db = await getDb();
	await db.execute('UPDATE tasks SET reminder_sent_at = $1 WHERE id = $2', [sentAt, id]);
}
