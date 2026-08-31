import type { TaskStatus } from '$lib/types/task';

export const statusConfig: Record<TaskStatus, { label: string }> = {
	todo: { label: 'Todo' },
	in_progress: { label: 'In progress' },
	done: { label: 'Done' },
	canceled: { label: 'Canceled' }
};

export const statusOrder: TaskStatus[] = ['todo', 'in_progress', 'done', 'canceled'];

export const priorityConfig: Record<number, { label: string; bars: number; color: string }> = {
	0: { label: 'No priority', bars: 0, color: 'text-muted-foreground' },
	1: { label: 'Urgent', bars: 3, color: 'text-red-400' },
	2: { label: 'High', bars: 3, color: 'text-orange-400' },
	3: { label: 'Medium', bars: 2, color: 'text-yellow-400' },
	4: { label: 'Low', bars: 1, color: 'text-blue-400' }
};
