import { getDb } from '$lib/db/client';

export type WeekBucket = {
	weekStart: string;
	created: number;
	done: number;
};

export type StatsTotals = {
	open: number;
	done: number;
	canceled: number;
	overdue: number;
	trackedSeconds: number;
	runningTimers: number;
	estimatedMinutes: number;
};

export type ProjectStat = {
	id: string;
	name: string;
	prefix: string;
	open: number;
	done: number;
};

// monday of the week containing the date, as YYYY-MM-DD
function weekStart(date: Date): string {
	const d = new Date(date);
	d.setHours(0, 0, 0, 0);
	d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
	return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

// created vs done per week for the last N weeks; done is read from the
// activity log since tasks carry no completed-at timestamp
export async function weeklyCounts(weeks = 8): Promise<WeekBucket[]> {
	const db = await getDb();
	const [createdRows, doneRows] = await Promise.all([
		db.select<{ d: string }[]>('SELECT created_at AS d FROM tasks WHERE deleted_at IS NULL'),
		db.select<{ d: string }[]>(
			`SELECT a.created_at AS d FROM activity_log a
			 JOIN tasks t ON t.id = a.task_id
			 WHERE a.action = 'status_changed' AND a.new_value = 'Done'`
		)
	]);

	const buckets: WeekBucket[] = [];
	const byWeek = new Map<string, WeekBucket>();
	for (let i = weeks - 1; i >= 0; i--) {
		const start = new Date();
		start.setDate(start.getDate() - i * 7);
		const key = weekStart(start);
		const bucket = { weekStart: key, created: 0, done: 0 };
		byWeek.set(key, bucket);
		buckets.push(bucket);
	}

	for (const { d } of createdRows) {
		const bucket = byWeek.get(weekStart(new Date(d.replace(' ', 'T'))));
		if (bucket) bucket.created++;
	}
	for (const { d } of doneRows) {
		const bucket = byWeek.get(weekStart(new Date(d.replace(' ', 'T'))));
		if (bucket) bucket.done++;
	}
	return buckets;
}

export async function totals(): Promise<StatsTotals> {
	const db = await getDb();
	const rows = await db.select<
		{
			open: number;
			done: number;
			canceled: number;
			overdue: number;
			tracked: number;
			running: number;
			estimated: number;
		}[]
	>(
		`SELECT
			SUM(CASE WHEN status NOT IN ('done', 'canceled') THEN 1 ELSE 0 END) AS open,
			SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END) AS done,
			SUM(CASE WHEN status = 'canceled' THEN 1 ELSE 0 END) AS canceled,
			SUM(CASE WHEN status NOT IN ('done', 'canceled') AND due_date IS NOT NULL AND due_date < date('now', 'localtime') THEN 1 ELSE 0 END) AS overdue,
			COALESCE(SUM(tracked_seconds), 0) AS tracked,
			SUM(CASE WHEN timer_started_at IS NOT NULL THEN 1 ELSE 0 END) AS running,
			COALESCE(SUM(estimate_minutes), 0) AS estimated
		 FROM tasks WHERE deleted_at IS NULL`
	);
	const row = rows[0];
	return {
		open: row?.open ?? 0,
		done: row?.done ?? 0,
		canceled: row?.canceled ?? 0,
		overdue: row?.overdue ?? 0,
		trackedSeconds: row?.tracked ?? 0,
		runningTimers: row?.running ?? 0,
		estimatedMinutes: row?.estimated ?? 0
	};
}

export type ProjectTime = {
	id: string;
	name: string;
	prefix: string;
	seconds: number;
};

// banked time per project; a running timer's live segment counts too
export async function trackedPerProject(): Promise<ProjectTime[]> {
	const db = await getDb();
	return await db.select<ProjectTime[]>(
		`SELECT p.id, p.name, p.prefix,
			COALESCE(SUM(t.tracked_seconds), 0) +
			COALESCE(SUM(CASE WHEN t.timer_started_at IS NOT NULL
				THEN MAX(0, CAST((julianday('now') - julianday(t.timer_started_at)) * 86400 AS INTEGER))
				ELSE 0 END), 0) AS seconds
		 FROM projects p
		 LEFT JOIN tasks t ON t.project_id = p.id AND t.deleted_at IS NULL
		 GROUP BY p.id
		 HAVING seconds > 0
		 ORDER BY seconds DESC`
	);
}

export async function projectCounts(): Promise<ProjectStat[]> {
	const db = await getDb();
	return await db.select<ProjectStat[]>(
		`SELECT p.id, p.name, p.prefix,
			SUM(CASE WHEN t.status NOT IN ('done', 'canceled') THEN 1 ELSE 0 END) AS open,
			SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) AS done
		 FROM projects p
		 LEFT JOIN tasks t ON t.project_id = p.id AND t.deleted_at IS NULL
		 GROUP BY p.id
		 ORDER BY open DESC, p.name`
	);
}
