import type { TaskStatus } from '$lib/types/task';

export const statusConfig: Record<TaskStatus, { label: string }> = {
	todo: { label: 'Todo' },
	in_progress: { label: 'In progress' },
	done: { label: 'Done' },
	canceled: { label: 'Canceled' }
};

export const statusOrder: TaskStatus[] = ['todo', 'in_progress', 'done', 'canceled'];

export const priorityConfig: Record<number, { label: string; bars: number }> = {
	0: { label: 'No priority', bars: 0 },
	1: { label: 'Urgent', bars: 3 },
	2: { label: 'High', bars: 3 },
	3: { label: 'Medium', bars: 2 },
	4: { label: 'Low', bars: 1 }
};
