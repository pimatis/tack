import { toast } from 'svelte-sonner';
import { restore } from '$lib/repositories/task.repository';
import { statusConfig } from '$lib/task/constants';
import { issueId } from '$lib/task/utils';
import { getSettings } from '$lib/stores/settings';
import { matches } from '../utils';
import type { PaletteContext, PaletteItem, PaletteProvider } from '../types';
import type { Task } from '$lib/types/task';

function toItem(task: Task, ctx: PaletteContext): PaletteItem {
	return {
		id: task.id,
		label: task.title,
		status: task.status,
		hint: issueId(task, ctx.projects, getSettings()),
		keywords: 'deleted trash',
		run: async () => {
			try {
				await restore(task.id);
				toast.success('Task restored');
				ctx.refresh?.();
			} catch {
				toast.error('Failed to restore task');
			}
		}
	};
}

// hidden until typed: deleted tasks are fallback results, not defaults.
// in-memory match only - the trashed list is small enough that fts is not
// worth a second query per keystroke
export const trashProvider: PaletteProvider = {
	id: 'trash',
	heading: 'Trash',
	search(query, ctx) {
		if (!query) return [];
		return ctx.trash
			.filter((task) =>
				matches(
					`${issueId(task, ctx.projects, getSettings())} ${task.title} ${statusConfig[task.status].label}`,
					query
				)
			)
			.map((task) => toItem(task, ctx));
	},
	resolve(itemId, ctx) {
		const task = ctx.trash.find((t) => t.id === itemId);
		if (!task) return null;
		return toItem(task, ctx);
	}
};
