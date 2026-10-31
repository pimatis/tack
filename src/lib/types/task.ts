export type TaskStatus = 'backlog' | 'todo' | 'in_progress' | 'done' | 'canceled';

export type TaskPriority = 0 | 1 | 2 | 3 | 4;

export type Task = {
	id: string;
	number: number;
	projectId?: string | null;
	title: string;
	description?: string | null;
	status: TaskStatus;
	priority: TaskPriority;
	dueDate?: string | null;
	endDate?: string | null;
	// utc iso timestamp for the reminder notification, null when unset
	reminderAt?: string | null;
	// set once the reminder notification has fired
	reminderSentAt?: string | null;
	// daily, weekly:<iso weekday 1-7> or monthly:<day 1-31>
	recurrenceRule?: string | null;
	// the occurrence this task was spawned from
	recurrenceParentId?: string | null;
	// parent task in the hierarchy; null when top-level
	parentId?: string | null;
	// planned effort in minutes, null when unset
	estimateMinutes?: number | null;
	// accumulated timer time in seconds
	trackedSeconds?: number;
	// utc iso timestamp while the timer is running, null when stopped
	timerStartedAt?: string | null;
	// computed in the list queries: an open task blocks this one / own children
	blocked?: boolean;
	childrenTotal?: number;
	childrenDone?: number;
	sortOrder?: number;
	pinned?: boolean;
	labelIds?: string[];
	createdAt: string;
	updatedAt: string;
	deletedAt?: string | null;
};
