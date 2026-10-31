import { goHomeThenDispatch, matches } from '../utils';
import type { PaletteItem, PaletteProvider } from '../types';

const ITEMS: PaletteItem[] = [
	{
		id: 'status-backlog',
		label: 'Backlog',
		status: 'backlog',
		keywords: 'filter status backlog unprioritized',
		run: () => goHomeThenDispatch('filter-by-status', 'backlog')
	},
	{
		id: 'status-todo',
		label: 'Todo',
		status: 'todo',
		keywords: 'filter status open to do',
		run: () => goHomeThenDispatch('filter-by-status', 'todo')
	},
	{
		id: 'status-in-progress',
		label: 'In progress',
		status: 'in_progress',
		keywords: 'filter status doing wip working',
		run: () => goHomeThenDispatch('filter-by-status', 'in_progress')
	},
	{
		id: 'status-done',
		label: 'Done',
		status: 'done',
		keywords: 'filter status completed finished',
		run: () => goHomeThenDispatch('filter-by-status', 'done')
	},
	{
		id: 'status-canceled',
		label: 'Canceled',
		status: 'canceled',
		keywords: 'filter status cancelled dropped',
		run: () => goHomeThenDispatch('filter-by-status', 'canceled')
	},
	{
		id: 'pinned',
		label: 'Pinned',
		icon: 'pin',
		keywords: 'filter pinned starred favorite',
		run: () => goHomeThenDispatch('filter-pinned')
	},
	{
		id: 'today',
		label: 'Today',
		icon: 'calendar',
		keywords: 'filter due today',
		run: () => goHomeThenDispatch('filter-today')
	},
	{
		id: 'upcoming',
		label: 'Upcoming',
		icon: 'clock',
		keywords: 'filter due week next soon',
		run: () => goHomeThenDispatch('filter-upcoming')
	},
	{
		id: 'overdue',
		label: 'Overdue',
		icon: 'alarm',
		keywords: 'filter late past due missed',
		run: () => goHomeThenDispatch('filter-overdue')
	},
	{
		id: 'priority-urgent',
		label: 'Urgent priority',
		icon: 'flag',
		keywords: 'filter priority p1 critical',
		run: () => goHomeThenDispatch('filter-by-priority', 1)
	},
	{
		id: 'priority-high',
		label: 'High priority',
		icon: 'flag',
		keywords: 'filter priority p2',
		run: () => goHomeThenDispatch('filter-by-priority', 2)
	},
	{
		id: 'priority-medium',
		label: 'Medium priority',
		icon: 'flag',
		keywords: 'filter priority p3 normal',
		run: () => goHomeThenDispatch('filter-by-priority', 3)
	},
	{
		id: 'priority-low',
		label: 'Low priority',
		icon: 'flag',
		keywords: 'filter priority p4 minor',
		run: () => goHomeThenDispatch('filter-by-priority', 4)
	},
	{
		id: 'clear',
		label: 'Clear filters',
		icon: 'close',
		keywords: 'reset remove all filters',
		run: () => goHomeThenDispatch('clear-filters')
	}
];

// hidden until typed or scoped ("?" lists them all): 13 flat rows would
// crowd the empty state
export const filtersProvider: PaletteProvider = {
	id: 'filters',
	heading: 'Filters',
	search(query, _ctx, scoped) {
		if (!query && !scoped) return [];
		return ITEMS.filter((item) => matches(`${item.label} ${item.keywords ?? ''}`, query));
	},
	resolve(itemId) {
		return ITEMS.find((item) => item.id === itemId) ?? null;
	}
};
