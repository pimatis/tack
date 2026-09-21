import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { goto } from '$app/navigation';
import { tick } from 'svelte';
import { isTauri } from '$lib/db/client';
import { notificationPermission } from '$lib/permissions/permissions.service';
import { findById, findDueReminders, markReminderSent } from '$lib/repositories/task.repository';
import { notesState } from '$lib/notes/notesState.svelte';

// reminders are checked on a coarse timer; a few seconds of latency is fine
const POLL_MS = 30_000;

// clicking a notification brings the window forward and emits the task id;
// switch to the tasks tab, make sure the home view is mounted, then open it
async function openTaskById(taskId: string): Promise<void> {
	const task = await findById(taskId).catch(() => null);
	if (!task) return;
	notesState.activeTab = 'tasks';
	if (window.location.pathname !== '/') {
		await goto('/');
		await tick();
	}
	window.dispatchEvent(new CustomEvent('edit-task-from-command', { detail: task }));
}

async function checkReminders(): Promise<void> {
	const due = await findDueReminders(new Date().toISOString()).catch(() => []);
	if (due.length === 0) return;
	// a denied or not-yet-granted permission would swallow the banner; leave
	// the reminder pending so it goes out once the user allows it. dev builds
	// report 'unsupported' and use the legacy path, which needs no permission
	const permission = await notificationPermission();
	if (permission !== 'granted' && permission !== 'unsupported') return;
	for (const reminder of due) {
		const issue = reminder.prefix ? `${reminder.prefix}-${reminder.number}` : `#${reminder.number}`;
		// title is the task name; the body carries the id and why it fired
		const body = reminder.dueDate ? `${issue} · due ${reminder.dueDate}` : `${issue} · reminder`;
		await invoke('notify_reminder', {
			taskId: reminder.id,
			title: reminder.title,
			body,
			subtitle: 'Task reminder'
		}).catch(() => {});
		await markReminderSent(reminder.id, new Date().toISOString()).catch(() => {});
	}
}

export function startReminderScheduler(): () => void {
	// notifications belong to the desktop app; live tabs must not each fire them
	if (!isTauri()) return () => {};
	void checkReminders();
	const interval = setInterval(() => void checkReminders(), POLL_MS);
	const unlisten: Promise<UnlistenFn> = listen<string>('reminder-clicked', (event) => {
		void openTaskById(event.payload);
	});
	return () => {
		clearInterval(interval);
		void unlisten.then((off) => off());
	};
}
