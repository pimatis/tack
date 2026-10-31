import { getDb } from '$lib/db/client';
import type { TaskPriority } from '$lib/types/task';

export type ExportData = {
	projects: unknown[];
	tasks: unknown[];
	labels: unknown[];
	taskLabels: unknown[];
	subtasks: unknown[];
	attachments: unknown[];
	activityLog: unknown[];
	taskRelations?: unknown[];
	savedViews?: unknown[];
	exportedAt: string;
};

export async function exportAll(): Promise<ExportData> {
	const db = await getDb();
	const [
		projects,
		tasks,
		labels,
		taskLabels,
		subtasks,
		attachments,
		activityLog,
		taskRelations,
		savedViews
	] = await Promise.all([
		db.select<unknown[]>('SELECT * FROM projects'),
		db.select<unknown[]>('SELECT * FROM tasks'),
		db.select<unknown[]>('SELECT * FROM labels'),
		db.select<unknown[]>('SELECT * FROM task_labels'),
		db.select<unknown[]>('SELECT * FROM subtasks'),
		db.select<unknown[]>(
			'SELECT id, task_id, file_name, file_path, mime_type, file_size, created_at FROM task_attachments'
		),
		db.select<unknown[]>('SELECT * FROM activity_log'),
		db.select<unknown[]>('SELECT * FROM task_relations').catch(() => [] as unknown[]),
		db.select<unknown[]>('SELECT * FROM saved_views').catch(() => [] as unknown[])
	]);

	return {
		projects,
		tasks,
		labels,
		taskLabels,
		subtasks,
		attachments,
		activityLog,
		taskRelations,
		savedViews,
		exportedAt: new Date().toISOString()
	};
}

export async function resetDatabase(): Promise<void> {
	const db = await getDb();
	const tables = [
		'activity_log',
		'task_relations',
		'task_labels',
		'task_attachments',
		'subtasks',
		'saved_views',
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
			`INSERT INTO tasks (id, number, project_id, title, description, status, priority, due_date, sort_order, pinned, created_at, updated_at, deleted_at, recurrence_rule, recurrence_parent_id, parent_id, estimate_minutes, tracked_seconds)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)`,
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
				row.deleted_at ?? null,
				row.recurrence_rule ?? null,
				row.recurrence_parent_id ?? null,
				row.parent_id ?? null,
				row.estimate_minutes ?? null,
				row.tracked_seconds ?? 0
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

	for (const r of data.taskRelations ?? []) {
		const row = r as Record<string, unknown>;
		await db.execute(
			'INSERT OR IGNORE INTO task_relations (task_id, related_task_id, type, created_at) VALUES ($1, $2, $3, $4)',
			[row.task_id, row.related_task_id, row.type, row.created_at]
		);
	}

	for (const v of data.savedViews ?? []) {
		const row = v as Record<string, unknown>;
		await db.execute(
			'INSERT INTO saved_views (id, name, filters, sort_order, created_at) VALUES ($1, $2, $3, $4, $5)',
			[row.id, row.name, row.filters, row.sort_order ?? 0, row.created_at]
		);
	}
}

// ---- csv + markdown export, csv import ----

type CsvTask = Record<string, unknown>;

const CSV_COLUMNS = [
	'title',
	'description',
	'status',
	'priority',
	'project',
	'labels',
	'due_date',
	'end_date',
	'estimate_minutes'
] as const;

function csvCell(value: unknown): string {
	const s = value == null ? '' : String(value);
	if (/[",\n]/.test(s)) return `"${s.replaceAll('"', '""')}"`;
	return s;
}

function parseCsvLine(line: string): string[] {
	const cells: string[] = [];
	let cur = '';
	let inQuotes = false;
	for (let i = 0; i < line.length; i++) {
		const c = line[i];
		if (inQuotes) {
			if (c === '"' && line[i + 1] === '"') {
				cur += '"';
				i++;
			} else if (c === '"') {
				inQuotes = false;
			} else {
				cur += c;
			}
		} else if (c === '"') {
			inQuotes = true;
		} else if (c === ',') {
			cells.push(cur);
			cur = '';
		} else {
			cur += c;
		}
	}
	cells.push(cur);
	return cells;
}

// tasks joined with their project prefix and label names, matching CSV_COLUMNS
async function csvTaskRows(db: Awaited<ReturnType<typeof getDb>>): Promise<CsvTask[]> {
	return await db.select<CsvTask[]>(
		`SELECT t.title, t.description, t.status, t.priority,
			p.prefix AS project,
			(SELECT GROUP_CONCAT(l.name, ';') FROM task_labels tl JOIN labels l ON l.id = tl.label_id WHERE tl.task_id = t.id) AS labels,
			t.due_date, t.end_date, t.estimate_minutes
		 FROM tasks t
		 LEFT JOIN projects p ON p.id = t.project_id
		 WHERE t.deleted_at IS NULL
		 ORDER BY t.created_at`
	);
}

export async function exportCsv(): Promise<string> {
	const db = await getDb();
	const rows = await csvTaskRows(db);
	const lines = [CSV_COLUMNS.join(',')];
	for (const row of rows) {
		lines.push(CSV_COLUMNS.map((c) => csvCell(row[c])).join(','));
	}
	return lines.join('\n') + '\n';
}

export async function exportMarkdown(): Promise<string> {
	const db = await getDb();
	const rows = await db.select<CsvTask[]>(
		`SELECT t.title, t.status, t.priority, t.due_date,
			p.prefix AS project, t.number
		 FROM tasks t
		 LEFT JOIN projects p ON p.id = t.project_id
		 WHERE t.deleted_at IS NULL
		 ORDER BY t.status, t.sort_order, t.created_at`
	);
	const checkbox = (s: string) => (s === 'done' ? '[x]' : s === 'canceled' ? '[-]' : '[ ]');
	const lines = ['# Tasks', ''];
	let lastStatus = '';
	for (const row of rows) {
		const status = String(row.status ?? 'todo');
		if (status !== lastStatus) {
			lines.push(`## ${status.replace('_', ' ')}`, '');
			lastStatus = status;
		}
		const id = `${row.project ?? 'TSK'}-${row.number}`;
		const due = row.due_date ? ` (due ${row.due_date})` : '';
		lines.push(`- ${checkbox(status)} **${id}** ${row.title}${due}`);
	}
	return lines.join('\n') + '\n';
}

// import tasks from the CSV_COLUMNS format; rows get fresh ids/numbers, projects
// and labels are matched by name/prefix and left out when unknown
export async function importCsv(text: string): Promise<number> {
	const { create } = await import('$lib/repositories/task.repository');
	const { findAll: findProjects } = await import('$lib/repositories/project.repository');
	const { findAll: findLabels, setTaskLabels } = await import('$lib/repositories/label.repository');

	const lines = text.split(/\r?\n/).filter((l) => l.trim().length > 0);
	if (lines.length === 0) return 0;
	const header = parseCsvLine(lines[0]).map((h) => h.trim().toLowerCase());
	const colIndex = (name: string) => header.indexOf(name);
	if (colIndex('title') === -1) throw new Error('CSV is missing a "title" column');

	const projects = await findProjects();
	const labels = await findLabels();
	const statusSet = new Set(['backlog', 'todo', 'in_progress', 'done', 'canceled']);

	let imported = 0;
	for (const line of lines.slice(1)) {
		const cells = parseCsvLine(line);
		const get = (name: string) => {
			const i = colIndex(name);
			return i === -1 ? '' : cells[i].trim();
		};
		const title = get('title');
		if (!title) continue;

		const projectKey = get('project').toLowerCase();
		const project = projectKey
			? projects.find(
					(p) => p.prefix.toLowerCase() === projectKey || p.name.toLowerCase() === projectKey
				)
			: undefined;
		const status = get('status').toLowerCase();
		const priority = Number(get('priority'));
		const estimate = Number(get('estimate_minutes'));

		const task = await create({
			title,
			description: get('description') || null,
			status: statusSet.has(status) ? (status as never) : 'todo',
			priority:
				Number.isInteger(priority) && priority >= 0 && priority <= 4
					? (priority as TaskPriority)
					: 0,
			projectId: project?.id ?? null,
			dueDate: get('due_date') || null,
			endDate: get('end_date') || null,
			estimateMinutes: Number.isInteger(estimate) && estimate > 0 ? estimate : null
		});

		const labelNames = get('labels')
			.split(';')
			.map((n) => n.trim().toLowerCase())
			.filter(Boolean);
		const labelIds = labels
			.filter((l) => labelNames.includes(l.name.toLowerCase()))
			.map((l) => l.id);
		if (labelIds.length > 0) await setTaskLabels(task.id, labelIds);
		imported++;
	}
	return imported;
}
