import { searchTaskIds } from '$lib/search/fts.service';
import { statusConfig } from '$lib/task/constants';
import { issueId } from '$lib/task/utils';
import { getSettings } from '$lib/stores/settings';
import { update, togglePin } from '$lib/repositories/task.repository';
import { goHomeThenDispatch } from '../utils';
import type { PaletteContext, PaletteItem, PaletteProvider } from '../types';
import type { Task } from '$lib/types/task';

function toItem(task: Task, ctx: PaletteContext): PaletteItem {
	return {
		id: task.id,
		label: task.title,
		status: task.status,
		hint: issueId(task, ctx.projects, getSettings()),
		actions: [
			{
				combo: 'mod-enter',
				hint: '⌘⏎',
				label: task.status === 'done' ? 'Reopen' : 'Done',
				run: async () => {
					try {
						await update(task.id, {
							status: task.status === 'done' ? 'todo' : 'done'
						});
						ctx.refresh?.();
					} catch {
						// ignore
					}
				}
			},
			{
				combo: 'alt-enter',
				hint: '⌥⏎',
				label: task.pinned ? 'Unpin' : 'Pin',
				run: async () => {
					try {
						await togglePin(task.id);
						ctx.refresh?.();
					} catch {
						// ignore
					}
				}
			}
		],
		run: () => goHomeThenDispatch('edit-task-from-command', task)
	};
}

// fts covers title, description, subtasks, labels, project and issue number;
// the in-memory checks add status names ("done" lists done tasks) and keep
// partial title/issue hits working while the async search is in flight
export const tasksProvider: PaletteProvider = {
	id: 'tasks',
	heading: 'Tasks',
	async search(query, ctx) {
		const q = query.toLowerCase();
		const ids = q ? await searchTaskIds(q).catch(() => null) : null;
		return ctx.tasks
			.filter((task) => {
				if (!q) return true;
				if (ids?.has(task.id)) return true;
				if (statusConfig[task.status].label.toLowerCase().includes(q)) return true;
				return `${issueId(task, ctx.projects, getSettings())} ${task.title}`
					.toLowerCase()
					.includes(q);
			})
			.map((task) => toItem(task, ctx));
	},
	resolve(itemId, ctx) {
		const task = ctx.tasks.find((t) => t.id === itemId);
		if (!task) return null;
		return toItem(task, ctx);
	}
};
