export type TaskStatus = 'todo' | 'in_progress' | 'done' | 'canceled';

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
	sortOrder?: number;
	pinned?: boolean;
	labelIds?: string[];
	createdAt: string;
	updatedAt: string;
	deletedAt?: string | null;
};
