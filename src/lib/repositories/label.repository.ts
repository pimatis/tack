import { getDb } from '$lib/db/client';
import type { Label, LabelColor } from '$lib/types/label';

export type CreateLabelInput = {
	name: string;
	color: LabelColor;
};

const COLUMNS = `
	id,
	name,
	color,
	created_at AS createdAt
`;

export async function create(input: CreateLabelInput): Promise<Label> {
	try {
		const db = await getDb();
		const id = crypto.randomUUID();
		const now = new Date().toISOString();
		const label: Label = { id, name: input.name, color: input.color, createdAt: now };

		await db.execute(`INSERT INTO labels (id, name, color, created_at) VALUES ($1, $2, $3, $4)`, [
			label.id,
			label.name,
			label.color,
			label.createdAt
		]);
		return label;
	} catch (error) {
		throw new Error('Failed to create label', { cause: error });
	}
}

export async function findAll(): Promise<Label[]> {
	try {
		const db = await getDb();
		return await db.select<Label[]>(`SELECT ${COLUMNS} FROM labels ORDER BY name COLLATE NOCASE`);
	} catch (error) {
		throw new Error('Failed to load labels', { cause: error });
	}
}

export async function update(id: string, input: Partial<CreateLabelInput>): Promise<Label | null> {
	try {
		const assignments: string[] = [];
		const values: unknown[] = [];

		if (input.name !== undefined) {
			values.push(input.name);
			assignments.push(`name = $${values.length}`);
		}
		if (input.color !== undefined) {
			values.push(input.color);
			assignments.push(`color = $${values.length}`);
		}

		if (assignments.length === 0) {
			const labels = await findAll();
			return labels.find((l) => l.id === id) ?? null;
		}

		values.push(id);
		const db = await getDb();
		const result = await db.execute(
			`UPDATE labels SET ${assignments.join(', ')} WHERE id = $${values.length}`,
			values
		);
		if (result.rowsAffected === 0) return null;

		const labels = await db.select<Label[]>(`SELECT ${COLUMNS} FROM labels WHERE id = $1`, [id]);
		return labels[0] ?? null;
	} catch (error) {
		throw new Error('Failed to update label', { cause: error });
	}
}

export async function remove(id: string): Promise<boolean> {
	try {
		const db = await getDb();
		const result = await db.execute('DELETE FROM labels WHERE id = $1', [id]);
		return result.rowsAffected > 0;
	} catch (error) {
		throw new Error('Failed to delete label', { cause: error });
	}
}

export async function setTaskLabels(taskId: string, labelIds: string[]): Promise<void> {
	try {
		const db = await getDb();
		await db.execute('DELETE FROM task_labels WHERE task_id = $1', [taskId]);
		for (const labelId of labelIds) {
			await db.execute('INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES ($1, $2)', [
				taskId,
				labelId
			]);
		}
	} catch (error) {
		throw new Error('Failed to set task labels', { cause: error });
	}
}

export async function findLabelIdsByTaskId(taskId: string): Promise<string[]> {
	try {
		const db = await getDb();
		const rows = await db.select<{ labelId: string }[]>(
			'SELECT label_id AS labelId FROM task_labels WHERE task_id = $1',
			[taskId]
		);
		return rows.map((r) => r.labelId);
	} catch (error) {
		throw new Error('Failed to load task labels', { cause: error });
	}
}

export async function findAllTaskLabelIds(): Promise<Map<string, string[]>> {
	try {
		const db = await getDb();
		const rows = await db.select<{ taskId: string; labelId: string }[]>(
			'SELECT task_id AS taskId, label_id AS labelId FROM task_labels'
		);
		const map = new Map<string, string[]>();
		for (const row of rows) {
			const existing = map.get(row.taskId) ?? [];
			existing.push(row.labelId);
			map.set(row.taskId, existing);
		}
		return map;
	} catch (error) {
		throw new Error('Failed to load task labels', { cause: error });
	}
}
