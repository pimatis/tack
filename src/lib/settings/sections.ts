import type { IconName } from '$lib/icons';

export type SettingsSection = {
	value: string;
	label: string;
	description: string;
	// semantic icon key, resolved via the registry (see src/lib/icons.ts)
	icon: IconName;
};

// single source of truth for the settings nav, the page's mobile picker and
// search labels, and the command palette's "Settings: ..." deep links
export const settingsSections: SettingsSection[] = [
	{
		value: 'appearance',
		label: 'Appearance',
		description: 'Theme, color and layout basics',
		icon: 'palette'
	},
	{
		value: 'themes',
		label: 'Themes',
		description: 'Pick a color palette for the whole app',
		icon: 'paint-brush'
	},
	{
		value: 'sidebar',
		label: 'Sidebar',
		description: 'Choose what shows in the sidebar',
		icon: 'panel-left'
	},
	{ value: 'tasks', label: 'Tasks', description: 'Defaults for new tasks', icon: 'checklist' },
	{
		value: 'data',
		label: 'Data',
		description: 'Import, export and reset your data',
		icon: 'database'
	},
	{
		value: 'backup',
		label: 'Backup',
		description: 'Automatic snapshots and restore points',
		icon: 'archive'
	},
	{
		value: 'shortcuts',
		label: 'Shortcuts',
		description: 'Keyboard shortcuts for common actions',
		icon: 'keyboard'
	},
	{
		value: 'live',
		label: 'Live',
		description: 'Share your workspace in a browser',
		icon: 'share'
	},
	{
		value: 'permissions',
		label: 'Permissions',
		description: 'System permissions tack can use',
		icon: 'notification'
	},
	{
		value: 'workspace',
		label: 'Workspace',
		description: 'Stats, CLI and about tack',
		icon: 'widget'
	}
];
