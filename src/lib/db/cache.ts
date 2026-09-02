import type { Task } from '$lib/types/task';
import type { Project } from '$lib/types/project';
import type { Label } from '$lib/types/label';

// stale-while-revalidate cache: serve the last known data instantly on
// startup, then let the db refresh overwrite it in the background
const STORAGE_KEY = 'tack-data-cache-v1';
const MAX_SIZE = 1_500_000; // skip caching past ~1.5MB so localStorage stays healthy

type CachedData = {
	tasks: Task[];
	projects: Project[];
	labels: Label[];
};

export function readDataCache(): CachedData | null {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return null;
		const parsed = JSON.parse(raw) as CachedData;
		if (
			!Array.isArray(parsed.tasks) ||
			!Array.isArray(parsed.projects) ||
			!Array.isArray(parsed.labels)
		)
			return null;
		return parsed;
	} catch {
		return null;
	}
}

export function writeDataCache(data: CachedData): void {
	try {
		const serialized = JSON.stringify(data);
		if (serialized.length > MAX_SIZE) return;
		localStorage.setItem(STORAGE_KEY, serialized);
	} catch {
		// storage full/unavailable - cache is best-effort
	}
}
