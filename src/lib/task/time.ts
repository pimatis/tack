import type { Task } from '$lib/types/task';

// parse a user-typed estimate into minutes: "90", "90m", "1h", "1h30m", "1.5h".
// returns null for empty input and unparseable values; the caller checks
// input.trim() first to tell "clear the estimate" apart from "invalid"
export function parseEstimate(input: string): number | null {
	const v = input.trim().toLowerCase().replace(/\s+/g, '');
	if (!v) return null;
	const hours = v.match(/^(\d+(?:\.\d+)?)h(?:(\d+)m?)?$/);
	if (hours) return Math.round(parseFloat(hours[1]) * 60 + (hours[2] ? Number(hours[2]) : 0));
	const minutes = v.match(/^(\d+)m?$/);
	if (minutes) return Number(minutes[1]);
	return null;
}

export function formatMinutes(minutes: number | null | undefined): string {
	if (!minutes || minutes <= 0) return '';
	const h = Math.floor(minutes / 60);
	const m = minutes % 60;
	if (h && m) return `${h}h ${m}m`;
	if (h) return `${h}h`;
	return `${m}m`;
}

export function formatSeconds(seconds: number): string {
	return formatMinutes(Math.round(seconds / 60)) || '0m';
}

// total tracked seconds including the live segment of a running timer
export function trackedSeconds(task: Task, now = Date.now()): number {
	let total = task.trackedSeconds ?? 0;
	if (task.timerStartedAt) {
		const started = new Date(task.timerStartedAt).getTime();
		total += Math.max(0, Math.floor((now - started) / 1000));
	}
	return total;
}
