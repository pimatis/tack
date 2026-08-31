export type Theme = 'dark' | 'light' | 'system';

export type SidebarItemId =
	'pinned' | 'today' | 'upcoming' | 'overdue' | 'status' | 'priority' | 'quickStats';

export type SidebarItemConfig = {
	id: SidebarItemId;
	visible: boolean;
};

export type Settings = {
	theme: Theme;
	sidebarCollapsed: boolean;
	defaultViewMode: 'list' | 'board' | 'calendar';
	defaultStatus: 'todo' | 'in_progress';
	defaultPriority: 0 | 1 | 2 | 3 | 4;
	dueSoonThreshold: number;
	prefixPadding: number;
	backupEnabled: boolean;
	backupIntervalHours: number;
	backupKeepCount: number;
	sidebarItems: SidebarItemConfig[];
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
	sidebarCollapsed: false,
	defaultViewMode: 'list',
	defaultStatus: 'todo',
	defaultPriority: 0,
	dueSoonThreshold: 1,
	prefixPadding: 0,
	backupEnabled: true,
	backupIntervalHours: 24,
	backupKeepCount: 7,
	sidebarItems: [...defaultSidebarItems]
};
