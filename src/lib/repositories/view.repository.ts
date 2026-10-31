import { getDb } from '$lib/db/client';
import { newId } from '$lib/utils';
import type { SavedView, ViewFilters } from '$lib/types/view';

type ViewRow = {
	id: string;
	name: string;
	filters: string;
	sort_order: number;
	created_at: string;
};

function toView(row: ViewRow): SavedView {
	let filters: ViewFilters = {};
	try {
		filters = JSON.parse(row.filters) as ViewFilters;
	} catch {
		// corrupt row: treat as empty filter set instead of dropping the view
	}
	return {
		id: row.id,
		name: row.name,
		filters,
		sortOrder: row.sort_order,
		createdAt: row.created_at
	};
}

export async function findAll(): Promise<SavedView[]> {
	try {
		const db = await getDb();
		const rows = await db.select<ViewRow[]>(
			'SELECT id, name, filters, sort_order, created_at FROM saved_views ORDER BY sort_order, created_at'
		);
		return rows.map(toView);
	} catch (error) {
		throw new Error('Failed to load saved views', { cause: error });
	}
}

export async function create(name: string, filters: ViewFilters): Promise<SavedView> {
	try {
		const db = await getDb();
		const view: SavedView = {
			id: newId(),
			name,
			filters,
			sortOrder: 0,
			createdAt: new Date().toISOString()
		};
		await db.execute(
			`INSERT INTO saved_views (id, name, filters, sort_order, created_at)
			 VALUES ($1, $2, $3, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM saved_views), $4)`,
			[view.id, view.name, JSON.stringify(filters), view.createdAt]
		);
		return view;
	} catch (error) {
		throw new Error('Failed to save view', { cause: error });
	}
}

export async function remove(id: string): Promise<void> {
	try {
		const db = await getDb();
		await db.execute('DELETE FROM saved_views WHERE id = $1', [id]);
	} catch (error) {
		throw new Error('Failed to delete view', { cause: error });
	}
}

export async function rename(id: string, name: string): Promise<void> {
	const trimmed = name.trim();
	if (!trimmed) return;
	try {
		const db = await getDb();
		await db.execute('UPDATE saved_views SET name = $1 WHERE id = $2', [trimmed, id]);
	} catch (error) {
		throw new Error('Failed to rename view', { cause: error });
	}
}

export async function reorder(orderedIds: string[]): Promise<void> {
	try {
		const db = await getDb();
		for (let i = 0; i < orderedIds.length; i++) {
			await db.execute('UPDATE saved_views SET sort_order = $1 WHERE id = $2', [i, orderedIds[i]]);
		}
	} catch (error) {
		throw new Error('Failed to reorder views', { cause: error });
	}
}
