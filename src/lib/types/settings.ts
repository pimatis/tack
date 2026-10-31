import { SHORTCUTS } from '$lib/shortcuts/shortcuts';
import type { ShortcutKey } from '$lib/shortcuts/shortcuts';
import { DEFAULT_COLOR_PRESET, type ColorPresetId } from '$lib/theme/presets';

export type Theme = 'dark' | 'light' | 'system';

export type SidebarItemId =
	'pinned' | 'today' | 'upcoming' | 'overdue' | 'status' | 'priority' | 'quickStats';

export type SidebarItemConfig = {
	id: SidebarItemId;
	visible: boolean;
};

export type Settings = {
	theme: Theme;
	// color palette preset; `default` is the built-in monochrome theme
	themePreset: ColorPresetId;
	sidebarCollapsed: boolean;
	defaultViewMode: 'list' | 'board' | 'calendar';
	defaultStatus: 'backlog' | 'todo' | 'in_progress';
	defaultPriority: 0 | 1 | 2 | 3 | 4;
	dueSoonThreshold: number;
	prefixPadding: number;
	backupEnabled: boolean;
	backupIntervalHours: number;
	backupKeepCount: number;
	liveEnabled: boolean;
	livePort: number;
	// shared password for the live server, stored as pbkdf2 hash + salt
	// (never plaintext); empty means no password is required
	livePasswordHash: string;
	livePasswordSalt: string;
	// optional second password that logs a visitor in with read-only access:
	// every read works, writes are refused
	liveReadPasswordHash: string;
	liveReadPasswordSalt: string;
	// loopback-only ui control for ai agents (see the agents settings tab)
	agentEnabled: boolean;
	agentPort: number;
	// tray "do not disturb": suppresses reminder notifications while true
	remindersMuted: boolean;
	// show the app icon in the menu bar / system tray
	trayEnabled: boolean;
	sidebarItems: SidebarItemConfig[];
	shortcuts: Record<string, ShortcutKey[]>;
};

export const defaultSidebarItems: SidebarItemConfig[] = [
	{ id: 'pinned', visible: true },
	{ id: 'today', visible: true },
	{ id: 'upcoming', visible: true },
	{ id: 'overdue', visible: true },
	{ id: 'status', visible: true },
	{ id: 'priority', visible: true },
	{ id: 'quickStats', visible: true }
];

export const defaultSettings: Settings = {
	theme: 'dark',
	themePreset: DEFAULT_COLOR_PRESET,
	sidebarCollapsed: false,
	defaultViewMode: 'list',
	defaultStatus: 'todo',
	defaultPriority: 0,
	dueSoonThreshold: 1,
	prefixPadding: 0,
	backupEnabled: true,
	backupIntervalHours: 24,
	backupKeepCount: 7,
	liveEnabled: false,
	livePort: 17890,
	livePasswordHash: '',
	livePasswordSalt: '',
	liveReadPasswordHash: '',
	liveReadPasswordSalt: '',
	agentEnabled: true,
	agentPort: 17891,
	remindersMuted: false,
	trayEnabled: true,
	sidebarItems: [...defaultSidebarItems],
	shortcuts: Object.fromEntries(SHORTCUTS.map((s) => [s.id, s.keys])) as Record<
		string,
		ShortcutKey[]
	>
};
