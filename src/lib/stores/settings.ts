import {
	defaultSettings,
	defaultSidebarItems,
	type Settings,
	type SidebarItemConfig,
	type SidebarItemId,
	type Theme
} from '$lib/types/settings';
import type { ShortcutKey } from '$lib/shortcuts/shortcuts';

const STORAGE_KEY = 'tack-settings';

let current: Settings = { ...defaultSettings };

// keys currently being written to the db - the sync poll must not overwrite them
const pendingPersist = new Set<string>();

// merge stored sidebar items with defaults so new items appear for existing users
function migrateSidebarItems(stored: SidebarItemConfig[] | undefined): SidebarItemConfig[] {
	if (!stored || !Array.isArray(stored)) return [...defaultSidebarItems];
	const knownIds = new Set<SidebarItemId>([
		'pinned',
		'today',
		'upcoming',
		'overdue',
		'status',
		'priority',
		'quickStats'
	]);
	const result: SidebarItemConfig[] = stored.filter((item) => knownIds.has(item.id));
	for (const def of defaultSidebarItems) {
		if (!result.some((item) => item.id === def.id)) {
			result.push({ ...def });
		}
	}
	return result;
}

function load(): Settings {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return { ...defaultSettings };
		const parsed = JSON.parse(raw);
		return {
			...defaultSettings,
			...parsed,
			sidebarItems: migrateSidebarItems(parsed.sidebarItems),
			// merge so newly added shortcuts appear with defaults for existing users
			shortcuts: { ...defaultSettings.shortcuts, ...(parsed.shortcuts ?? {}) }
		};
	} catch {
		return { ...defaultSettings };
	}
}

function save(settings: Settings): void {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
	} catch {
		// ignore quota errors
	}
}

export function getSettings(): Settings {
	return current;
}

export function setSettings(updates: Partial<Settings>): Settings {
	current = { ...current, ...updates };
	save(current);
	applyTheme(current.theme);
	persistToDb(updates);
	window.dispatchEvent(new Event('settings-changed'));
	return current;
}

// mirror changed settings into the db so the cli (tack settings get/set) stays in sync
function persistToDb(updates: Partial<Settings>): void {
	void (async () => {
		try {
			// dynamic import keeps tauri modules out of the server-side bundle
			const { getDb } = await import('$lib/db/client');
			const db = await getDb();
			for (const [key, value] of Object.entries(updates)) {
				const str = typeof value === 'object' ? JSON.stringify(value) : String(value);
				pendingPersist.add(key);
				await db.execute(
					'INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = $2',
					[key, str]
				);
				pendingPersist.delete(key);
			}
		} catch {
			// db unavailable (browser dev) - keep localStorage as source of truth
			pendingPersist.clear();
		}
	})();
}

function parseDbValue(key: string, value: string): unknown {
	if (key === 'sidebarItems' || key === 'shortcuts') return JSON.parse(value);
	if (value === 'true') return true;
	if (value === 'false') return false;
	const num = Number(value);
	if (value !== '' && !Number.isNaN(num)) return num;
	return value;
}

// apply settings changed via the cli (tack settings set) on top of localStorage
export async function loadSettingsFromDb(): Promise<void> {
	try {
		// dynamic import keeps tauri modules out of the server-side bundle
		const { getDb } = await import('$lib/db/client');
		const db = await getDb();
		const rows = await db.select<{ key: string; value: string }[]>(
			'SELECT key, value FROM settings'
		);
		const merged: Partial<Settings> = {};
		for (const row of rows) {
			if (!(row.key in current) || pendingPersist.has(row.key)) continue;
			try {
				const parsed = parseDbValue(row.key, row.value);
				if (row.key === 'shortcuts') {
					// merge so cli-set shortcuts never drop the other defaults
					merged.shortcuts = {
						...defaultSettings.shortcuts,
						...(parsed as Record<string, ShortcutKey[]>)
					};
				} else if (row.key === 'sidebarItems') {
					merged.sidebarItems = migrateSidebarItems(parsed as SidebarItemConfig[] | undefined);
				} else {
					merged[row.key as keyof Settings] = parsed as never;
				}
			} catch {
				// skip unparsable values
			}
		}
		// only apply + notify when something actually changed
		const changed = Object.entries(merged).some(
			([key, value]) => JSON.stringify(current[key as keyof Settings]) !== JSON.stringify(value)
		);
		if (!changed) return;
		current = { ...current, ...merged };
		save(current);
		applyTheme(current.theme);
		window.dispatchEvent(new Event('settings-changed'));
	} catch {
		// db unavailable (browser dev)
	}
}

export function initSettings(): Settings {
	current = load();
	// skip on server render where window/document are unavailable
	if (typeof window !== 'undefined') applyTheme(current.theme);
	return current;
}

export function applyTheme(theme: Theme): void {
	const root = document.documentElement;
	const isDark =
		theme === 'dark' ||
		(theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
	root.classList.toggle('dark', isDark);
}
