import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { goto } from '$app/navigation';
import { tick } from 'svelte';
import { isTauri, onDbChanged } from '$lib/db/client';
import { findAll } from '$lib/repositories/task.repository';
import { getSettings, setSettings } from '$lib/stores/settings';
import { openTaskById } from '$lib/reminders/reminder.service';
import type { Task } from '$lib/types/task';

// same calendar-span rules the task list's today/overdue filters apply:
// dueDate is the span start, endDate the span end, today is a local midnight
function span(t: Task): { start: Date | null; end: Date | null } {
	return {
		start: t.dueDate ? new Date(t.dueDate + 'T00:00:00') : null,
		end: t.endDate ? new Date(t.endDate + 'T00:00:00') : null
	};
}

function openTasks(tasks: Task[]): Task[] {
	return tasks.filter((t) => t.status !== 'done' && t.status !== 'canceled');
}

// badge and submenu cover the union of overdue and due-today open tasks
function dueTasks(tasks: Task[]): Task[] {
	const now = new Date();
	now.setHours(0, 0, 0, 0);
	return openTasks(tasks).filter((t) => {
		const { start, end } = span(t);
		if (!start && !end) return false;
		const overdue =
			(start && start.getTime() < now.getTime()) || (end && end.getTime() < now.getTime());
		const today =
			(!start || start.getTime() <= now.getTime()) && (!end || end.getTime() >= now.getTime());
		return overdue || today;
	});
}

async function sync(): Promise<void> {
	const tasks = dueTasks(await findAll().catch(() => []));
	const byEnd = (a: Task, b: Task) =>
		(a.endDate ?? a.dueDate ?? '').localeCompare(b.endDate ?? b.dueDate ?? '');
	const settings = getSettings();
	await invoke('update_tray', {
		count: tasks.length,
		tasks: tasks
			.sort(byEnd)
			.slice(0, 10)
			.map((t) => ({ id: t.id, title: t.title })),
		liveOn: settings.liveEnabled,
		dndOn: settings.remindersMuted,
		trayOn: settings.trayEnabled
	}).catch(() => {});
}

// tray filters only exist on the home page; navigate first, then dispatch
async function navigateToFilter(filter: 'today' | 'overdue'): Promise<void> {
	await goto('/');
	await tick();
	window.dispatchEvent(new CustomEvent(`filter-${filter}`));
}

async function quickAddFromClipboard(): Promise<void> {
	const text = await navigator.clipboard.readText().catch(() => '');
	await goto('/');
	await tick();
	window.dispatchEvent(
		new CustomEvent('open-task-dialog', { detail: { title: text.trim() || null } })
	);
}

// owns the tray's runtime state: pushes badge/submenu/checks on every data or
// settings change, answers menu clicks with ui events. desktop-only
export function startTrayService(): () => void {
	if (!isTauri()) return () => {};

	const unlistens: Promise<UnlistenFn>[] = [
		listen<'today' | 'overdue'>('tray-navigate', (e) => void navigateToFilter(e.payload)),
		listen<string>('focus-task', (e) => void openTaskById(e.payload)),
		listen('quick-add-clipboard', () => void quickAddFromClipboard()),
		listen('tray-toggle-live', () => setSettings({ liveEnabled: !getSettings().liveEnabled })),
		listen('tray-toggle-dnd', () => setSettings({ remindersMuted: !getSettings().remindersMuted })),
		listen('check-updates', () => window.dispatchEvent(new Event('open-update-dialog')))
	];
	const offDbChanged = onDbChanged(() => void sync());
	const onSettings = () => void sync();
	window.addEventListener('settings-changed', onSettings);
	void sync();

	return () => {
		offDbChanged();
		window.removeEventListener('settings-changed', onSettings);
		for (const unlisten of unlistens) void unlisten.then((off) => off());
	};
}
