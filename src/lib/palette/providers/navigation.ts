import { goto } from '$app/navigation';
import { settingsSections } from '$lib/settings/sections';
import { goHomeThenDispatch, matches } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';

function buildItems(): PaletteItem[] {
	return [
		{
			id: 'home',
			label: 'Home',
			icon: 'home',
			keywords: 'tasks main page go to',
			run: () => goto('/')
		},
		{
			id: 'stats',
			label: 'Stats',
			icon: 'chart',
			keywords: 'stats analytics report page go to',
			run: () => goto('/stats')
		},
		{
			id: 'trash',
			label: 'Trash',
			icon: 'trash',
			keywords: 'deleted bin page go to',
			run: () => goto('/trash')
		},
		{
			id: 'settings',
			label: 'Settings',
			icon: 'settings',
			keywords: 'preferences options page go to',
			run: () => goto('/settings')
		},
		...settingsSections.map((section) => ({
			id: `settings-${section.value}`,
			label: `Settings: ${section.label}`,
			icon: section.icon,
			keywords: `settings preferences ${section.description}`,
			run: () => goto(`/settings?tab=${section.value}`)
		})),
		{
			id: 'view-list',
			label: 'List view',
			icon: 'list',
			keywords: 'view layout tasks switch',
			run: () => goHomeThenDispatch('set-view', 'list')
		},
		{
			id: 'view-board',
			label: 'Board view',
			icon: 'kanban',
			keywords: 'view layout board kanban switch',
			run: () => goHomeThenDispatch('set-view', 'board')
		},
		{
			id: 'view-calendar',
			label: 'Calendar view',
			icon: 'calendar',
			keywords: 'view layout calendar schedule switch',
			run: () => goHomeThenDispatch('set-view', 'calendar')
		}
	];
}

// hidden until typed: navigation targets are fallback results, not defaults
export const navigationProvider: PaletteProvider = {
	id: 'navigation',
	heading: 'Go to',
	search(query) {
		if (!query) return [];
		return buildItems().filter((item) => matches(`${item.label} ${item.keywords ?? ''}`, query));
	},
	resolve(itemId) {
		return buildItems().find((item) => item.id === itemId) ?? null;
	}
};
