import { toast } from 'svelte-sonner';
import { getSettings, setSettings } from '$lib/stores/settings';
import { createBackup } from '$lib/backup/backup.service';
import { checkForUpdate } from '$lib/updater/update.service';
import { create, togglePin } from '$lib/repositories/task.repository';
import { setTaskLabels } from '$lib/repositories/label.repository';
import { parseQuickAdd, type ParsedQuickAdd } from '$lib/quickadd/parse';
import { priorityConfig } from '$lib/task/constants';
import { recurrenceLabel } from '$lib/task/recurrence';
import { goHomeThenDispatch, goThenDispatch, matches, shortcutHint } from '../utils';
import type { PaletteContext, PaletteItem, PaletteProvider } from '../types';
import type { TaskPriority } from '$lib/types/task';

function buildItems(): PaletteItem[] {
	return [
		{
			id: 'new-task',
			label: 'New task',
			icon: 'compose',
			hint: shortcutHint('new-task', 'C'),
			keywords: 'create add todo',
			run: () => goHomeThenDispatch('open-task-dialog')
		},
		{
			id: 'new-project',
			label: 'New project',
			icon: 'add',
			hint: shortcutHint('new-project', 'N'),
			keywords: 'create add',
			run: () => goHomeThenDispatch('open-project-dialog')
		},
		{
			id: 'toggle-sidebar',
			label: 'Toggle sidebar',
			icon: 'panel-left',
			hint: shortcutHint('toggle-sidebar', ''),
			keywords: 'sidebar collapse expand panel',
			run: () => {
				setSettings({ sidebarCollapsed: !getSettings().sidebarCollapsed });
			}
		},
		{
			id: 'backup-now',
			label: 'Backup now',
			icon: 'archive',
			keywords: 'backup save snapshot export data',
			run: () => {
				void createBackup(getSettings().backupKeepCount)
					.then(() => toast.success('Backup created'))
					.catch(() => toast.error('Backup failed'));
			}
		},
		{
			id: 'toggle-live',
			label: 'Toggle live server',
			icon: 'share',
			hint: getSettings().liveEnabled ? 'On' : 'Off',
			keywords: 'live share server browser broadcast',
			run: () => {
				setSettings({ liveEnabled: !getSettings().liveEnabled });
			}
		},
		{
			id: 'check-updates',
			label: 'Check for updates',
			icon: 'download',
			keywords: 'update version upgrade release',
			run: () => {
				void checkForUpdate()
					.then((update) => {
						if (update) toast.info(`Update available: v${update.version}`);
						else toast.success('You are up to date');
					})
					.catch(() => toast.error('Update check failed'));
			}
		},
		{
			id: 'empty-trash',
			label: 'Empty trash',
			icon: 'trash',
			keywords: 'delete trash clear permanently',
			run: () => goThenDispatch('/trash', 'open-empty-trash-confirm')
		}
	];
}

function findProject(parsed: ParsedQuickAdd, ctx: PaletteContext) {
	if (!parsed.projectKey) return undefined;
	return ctx.projects.find(
		(p) =>
			p.prefix.toLowerCase() === parsed.projectKey || p.name.toLowerCase() === parsed.projectKey
	);
}

function findLabel(name: string, ctx: PaletteContext) {
	return ctx.labels.find(
		(l) => l.name.toLowerCase() === name || l.name.toLowerCase().replace(/ /g, '-') === name
	);
}

// compact right-side preview of what the line parsed into; unresolved
// projects/labels are marked so the user sees the miss before hitting enter
function quickAddHint(parsed: ParsedQuickAdd, ctx: PaletteContext): string {
	const parts: string[] = [];
	if (parsed.projectKey) {
		const project = findProject(parsed, ctx);
		parts.push(project ? project.prefix : `#${parsed.projectKey}?`);
	}
	for (const name of parsed.labelNames) {
		const label = findLabel(name, ctx);
		parts.push(label ? label.name : `+${name}?`);
	}
	if (parsed.priority !== null) parts.push(priorityConfig[parsed.priority].label);
	if (parsed.pinned) parts.push('Pinned');
	if (parsed.dueDate) parts.push(`due ${parsed.dueDate}`);
	if (parsed.endDate) parts.push(`end ${parsed.endDate}`);
	if (parsed.reminderAt) {
		const d = new Date(parsed.reminderAt);
		const p = (n: number) => String(n).padStart(2, '0');
		parts.push(
			`remind ${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
		);
	}
	if (parsed.recurrenceRule) parts.push(recurrenceLabel(parsed.recurrenceRule) ?? '');
	if (parsed.estimateMinutes !== null) parts.push(`~${parsed.estimateMinutes}m`);
	return parts.filter(Boolean).join(' · ');
}

async function runQuickAdd(parsed: ParsedQuickAdd, query: string, ctx: PaletteContext) {
	const title = parsed.title || query;
	const project = findProject(parsed, ctx);
	if (parsed.projectKey && !project) {
		toast.error(`No project matches "#${parsed.projectKey}"`);
		return;
	}
	const missing: string[] = [];
	const labelIds: string[] = [];
	for (const name of parsed.labelNames) {
		const label = findLabel(name, ctx);
		if (label) labelIds.push(label.id);
		if (!label) missing.push(name);
	}
	try {
		let task = await create({
			title,
			projectId: project?.id ?? null,
			status: getSettings().defaultStatus,
			priority: (parsed.priority ?? getSettings().defaultPriority) as TaskPriority,
			dueDate: parsed.dueDate,
			endDate: parsed.endDate,
			reminderAt: parsed.reminderAt,
			recurrenceRule: parsed.recurrenceRule,
			estimateMinutes: parsed.estimateMinutes
		});
		if (labelIds.length) {
			await setTaskLabels(task.id, labelIds);
			task.labelIds = labelIds;
		}
		if (parsed.pinned) task = (await togglePin(task.id)) ?? task;
		// insert into the open task list immediately instead of waiting for the
		// db file watcher to round-trip a db-changed event
		window.dispatchEvent(new CustomEvent('task-created', { detail: task }));
		toast.success(
			missing.length ? `Task created, unknown labels: ${missing.join(', ')}` : 'Task created',
			{
				action: {
					label: 'Undo',
					onClick: () =>
						window.dispatchEvent(new CustomEvent('delete-task-from-panel', { detail: task.id }))
				}
			}
		);
		ctx.refresh?.();
	} catch {
		toast.error('Failed to create task');
	}
}

export const actionsProvider: PaletteProvider = {
	id: 'actions',
	heading: 'Actions',
	search(query, ctx) {
		const items = buildItems().filter((item) =>
			matches(`${item.label} ${item.keywords ?? ''}`, query)
		);
		// fallback row: the query parses into a full task spec and creates
		// directly; alt+enter falls back to the full dialog with just the title
		if (query) {
			const parsed = parseQuickAdd(query);
			items.push({
				id: 'quick-add',
				label: `Create task "${parsed.title || query}"`,
				icon: 'compose',
				hint: quickAddHint(parsed, ctx),
				noRecent: true,
				actions: [
					{
						combo: 'alt-enter',
						hint: '⌥⏎',
						label: 'Open in dialog',
						// carries every parsed field into the dialog, not just the title
						run: () => {
							const labelIds = parsed.labelNames
								.map((n) => findLabel(n, ctx)?.id)
								.filter((id): id is string => !!id);
							goHomeThenDispatch('open-task-dialog', {
								title: parsed.title || query,
								projectId: findProject(parsed, ctx)?.id,
								priority: parsed.priority ?? undefined,
								dueDate: parsed.dueDate,
								endDate: parsed.endDate,
								reminderAt: parsed.reminderAt,
								recurrenceRule: parsed.recurrenceRule,
								estimateMinutes: parsed.estimateMinutes,
								labelIds: labelIds.length ? labelIds : undefined
							});
						}
					}
				],
				run: () => runQuickAdd(parsed, query, ctx)
			});
		}
		return items;
	},
	resolve(itemId) {
		return buildItems().find((item) => item.id === itemId) ?? null;
	}
};
