import { getDb } from '$lib/db/client';

export type ExportData = {
	projects: unknown[];
	tasks: unknown[];
	labels: unknown[];
	taskLabels: unknown[];
	subtasks: unknown[];
	attachments: unknown[];
	activityLog: unknown[];
	exportedAt: string;
};

export async function exportAll(): Promise<ExportData> {
	const db = await getDb();
	const [projects, tasks, labels, taskLabels, subtasks, attachments, activityLog] =
		await Promise.all([
			db.select<unknown[]>('SELECT * FROM projects'),
			db.select<unknown[]>('SELECT * FROM tasks'),
			db.select<unknown[]>('SELECT * FROM labels'),
			db.select<unknown[]>('SELECT * FROM task_labels'),
			db.select<unknown[]>('SELECT * FROM subtasks'),
			db.select<unknown[]>(
				'SELECT id, task_id, file_name, file_path, mime_type, file_size, created_at FROM task_attachments'
			),
			db.select<unknown[]>('SELECT * FROM activity_log')
		]);

	return {
		projects,
		tasks,
		labels,
		taskLabels,
		subtasks,
		attachments,
		activityLog,
		exportedAt: new Date().toISOString()
	};
}

export async function resetDatabase(): Promise<void> {
	const db = await getDb();
	const tables = [
		'activity_log',
		'task_labels',
		'task_attachments',
		'subtasks',
		'tasks',
		'labels',
		'projects'
	];
	for (const table of tables) {
		await db.execute(`DELETE FROM ${table}`);
	}
}

export async function importData(data: ExportData): Promise<void> {
	const db = await getDb();
	await resetDatabase();

	for (const p of data.projects ?? []) {
		const row = p as Record<string, unknown>;
		await db.execute(
			'INSERT INTO projects (id, name, prefix, description, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)',
			[row.id, row.name, row.prefix, row.description ?? null, row.created_at, row.updated_at]
		);
	}

	for (const t of data.tasks ?? []) {
		const row = t as Record<string, unknown>;
		await db.execute(
			`INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, sort_order, pinned, created_at, updated_at, deleted_at)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
			[
				row.id,
				row.number,
				row.project_id ?? null,
				row.title,
				row.description ?? null,
				row.status,
				row.priority,
				row.due_date ?? null,
				row.sort_order ?? 0,
				row.pinned ?? 0,
				row.created_at,
				row.updated_at,
				row.deleted_at ?? null
			]
		);
	}

	for (const l of data.labels ?? []) {
		const row = l as Record<string, unknown>;
		await db.execute('INSERT INTO labels (id, name, color, created_at) VALUES ($1, $2, $3, $4)', [
			row.id,
			row.name,
			row.color,
			row.created_at
		]);
	}

	for (const tl of data.taskLabels ?? []) {
		const row = tl as Record<string, unknown>;
		await db.execute('INSERT INTO task_labels (task_id, label_id) VALUES ($1, $2)', [
			row.task_id,
			row.label_id
		]);
	}

	for (const s of data.subtasks ?? []) {
		const row = s as Record<string, unknown>;
		await db.execute(
			'INSERT INTO subtasks (id, task_id, title, completed, sort_order, created_at) VALUES ($1, $2, $3, $4, $5, $6)',
			[row.id, row.task_id, row.title, row.completed ?? 0, row.sort_order ?? 0, row.created_at]
		);
	}

	for (const a of data.attachments ?? []) {
		const row = a as Record<string, unknown>;
		await db.execute(
			'INSERT INTO task_attachments (id, task_id, file_name, file_path, mime_type, file_size, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)',
			[
				row.id,
				row.task_id,
				row.file_name,
				row.file_path ?? '',
				row.mime_type,
				row.file_size,
				row.created_at
			]
		);
	}

	for (const al of data.activityLog ?? []) {
		const row = al as Record<string, unknown>;
		await db.execute(
			'INSERT INTO activity_log (id, task_id, action, field, old_value, new_value, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)',
			[
				row.id,
				row.task_id,
				row.action,
				row.field ?? null,
				row.old_value ?? null,
				row.new_value ?? null,
				row.created_at
			]
		);
	}
}
