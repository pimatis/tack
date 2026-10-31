import { toast } from 'svelte-sonner';
import { getSettings, setSettings } from '$lib/stores/settings';
import { createBackup } from '$lib/backup/backup.service';
import { checkForUpdate } from '$lib/updater/update.service';
import { goHomeThenDispatch, goThenDispatch, matches, shortcutHint } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';

function buildItems(): PaletteItem[] {
	return [
		{
			id: 'new-task',
			label: 'New task',
			icon: 'compose',
			hint: shortcutHint('new-task', 'C'),
			keywords: 'create add todo',
			run: () => goHomeThenDispatch('open-task-dialog')
		},
		{
			id: 'new-project',
			label: 'New project',
			icon: 'add',
			hint: shortcutHint('new-project', 'N'),
			keywords: 'create add',
			run: () => goHomeThenDispatch('open-project-dialog')
		},
		{
			id: 'toggle-sidebar',
			label: 'Toggle sidebar',
			icon: 'panel-left',
			hint: shortcutHint('toggle-sidebar', ''),
			keywords: 'sidebar collapse expand panel',
			run: () => {
				setSettings({ sidebarCollapsed: !getSettings().sidebarCollapsed });
			}
		},
		{
			id: 'backup-now',
			label: 'Backup now',
			icon: 'archive',
			keywords: 'backup save snapshot export data',
			run: () => {
				void createBackup(getSettings().backupKeepCount)
					.then(() => toast.success('Backup created'))
					.catch(() => toast.error('Backup failed'));
			}
		},
		{
			id: 'toggle-live',
			label: 'Toggle live server',
			icon: 'share',
			hint: getSettings().liveEnabled ? 'On' : 'Off',
			keywords: 'live share server browser broadcast',
			run: () => {
				setSettings({ liveEnabled: !getSettings().liveEnabled });
			}
		},
		{
			id: 'check-updates',
			label: 'Check for updates',
			icon: 'download',
			keywords: 'update version upgrade release',
			run: () => {
				void checkForUpdate()
					.then((update) => {
						if (update) toast.info(`Update available: v${update.version}`);
						else toast.success('You are up to date');
					})
					.catch(() => toast.error('Update check failed'));
			}
		},
		{
			id: 'empty-trash',
			label: 'Empty trash',
			icon: 'trash',
			keywords: 'delete trash clear permanently',
			run: () => goThenDispatch('/trash', 'open-empty-trash-confirm')
		}
	];
}

export const actionsProvider: PaletteProvider = {
	id: 'actions',
	heading: 'Actions',
	search(query) {
		const items = buildItems().filter((item) =>
			matches(`${item.label} ${item.keywords ?? ''}`, query)
		);
		// fallback row: turn the query into a new task's title
		if (query) {
			items.push({
				id: 'create-task',
				label: `Create task "${query}"`,
				icon: 'compose',
				noRecent: true,
				run: () => goHomeThenDispatch('open-task-dialog', { title: query })
			});
		}
		return items;
	},
	resolve(itemId) {
		return buildItems().find((item) => item.id === itemId) ?? null;
	}
};
