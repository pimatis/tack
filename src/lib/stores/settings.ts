import {
	defaultSettings,
	defaultSidebarItems,
	type Settings,
	type SidebarItemConfig,
	type SidebarItemId,
	type Theme
} from '$lib/types/settings';

const STORAGE_KEY = 'tack-settings';

let current: Settings = { ...defaultSettings };

// merge stored sidebar items with defaults so new items appear for existing users
function migrateSidebarItems(
	stored: SidebarItemConfig[] | undefined
): SidebarItemConfig[] {
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
			sidebarItems: migrateSidebarItems(parsed.sidebarItems)
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
	window.dispatchEvent(new Event('settings-changed'));
	return current;
}

export function initSettings(): Settings {
	current = load();
	applyTheme(current.theme);
	return current;
}

export function applyTheme(theme: Theme): void {
	const root = document.documentElement;
	const isDark =
		theme === 'dark' ||
		(theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
	root.classList.toggle('dark', isDark);
}
