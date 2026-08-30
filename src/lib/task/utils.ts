import type { Task, TaskStatus } from '$lib/types/task';
import type { Project } from '$lib/types/project';
import type { Settings } from '$lib/types/settings';
import { statusOrder } from './constants';

export function isTypingTarget() {
	const el = document.activeElement as HTMLElement | null;
	if (!el) return false;
	return el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable;
}

export function dueDateInfo(dateStr: string | null | undefined): {
	label: string;
	textColor: string;
	bgColor: string;
	urgent: boolean;
	overdue: boolean;
} | null {
	if (!dateStr) return null;
	const due = new Date(dateStr + 'T00:00:00');
	const now = new Date();
	now.setHours(0, 0, 0, 0);
	const diffMs = due.getTime() - now.getTime();
	const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

	if (diffDays < 0)
		return {
			label: diffDays === -1 ? '1 day overdue' : `${Math.abs(diffDays)}d overdue`,
			textColor: 'text-red-400',
			bgColor: 'bg-red-500/10 border-red-500/20',
			urgent: true,
			overdue: true
		};
	if (diffDays === 0)
		return {
			label: 'Due today',
			textColor: 'text-amber-400',
			bgColor: 'bg-amber-500/10 border-amber-500/20',
			urgent: true,
			overdue: false
		};
	if (diffDays === 1)
		return {
			label: 'Due tomorrow',
			textColor: 'text-amber-400',
			bgColor: 'bg-amber-500/10 border-amber-500/20',
			urgent: true,
			overdue: false
		};
	if (diffDays <= 3)
		return {
			label: `In ${diffDays} days`,
			textColor: 'text-amber-400/80',
			bgColor: 'bg-amber-500/5 border-amber-500/10',
			urgent: false,
			overdue: false
		};

	const formatted = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(
		due
	);
	return {
		label: formatted,
		textColor: 'text-muted-foreground',
		bgColor: 'bg-transparent border-transparent',
		urgent: false,
		overdue: false
	};
}

export function issueId(task: Task, projects: Project[], appSettings: Settings): string {
	const project = projects.find((p) => p.id === task.projectId);
	const prefix = project?.prefix ?? 'TSK';
	const pad = appSettings.prefixPadding;
	return `${prefix}-${pad > 0 ? String(task.number).padStart(pad, '0') : task.number}`;
}

export function formatDate(value: string) {
	return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(new Date(value));
}

export function groupedTasks(
	tasks: Task[],
	statusOrderList: TaskStatus[] = statusOrder
): Record<TaskStatus, Task[]> {
	const groups: Record<TaskStatus, Task[]> = {
		todo: [],
		in_progress: [],
		done: [],
		canceled: []
	};
	for (const task of tasks) groups[task.status].push(task);
	for (const status of statusOrderList) {
		groups[status].sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
	}
	return groups;
}
