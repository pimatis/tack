import type { TaskStatus } from './task';

// the filter snapshot a saved view restores; projectIds may contain '' for
// tasks with no project
export type ViewFilters = {
	statuses?: TaskStatus[];
	projectIds?: string[];
	labelIds?: string[];
	priority?: number | null;
	pinned?: boolean;
	due?: 'today' | 'upcoming' | 'overdue' | null;
	updatedSinceDays?: number | null;
};

export type SavedView = {
	id: string;
	name: string;
	filters: ViewFilters;
	sortOrder: number;
	createdAt: string;
};
