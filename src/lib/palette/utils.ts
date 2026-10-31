import { tick } from 'svelte';
import { goto } from '$app/navigation';
import { getSettings } from '$lib/stores/settings';
import { keyComboLabel } from '$lib/shortcuts/index.js';

// palette actions act on a specific page: navigate there first, then let the
// page handle the event (open dialog, apply filter, switch view)
export async function goThenDispatch(path: string, eventName: string, detail?: unknown) {
	if (window.location.pathname !== path) {
		await goto(path);
		await tick();
	}
	if (detail !== undefined) {
		window.dispatchEvent(new CustomEvent(eventName, { detail }));
		return;
	}
	window.dispatchEvent(new Event(eventName));
}

export function goHomeThenDispatch(eventName: string, detail?: unknown) {
	return goThenDispatch('/', eventName, detail);
}

// substring match for provider-local filtering; both sides are normalized
export function matches(text: string, query: string): boolean {
	return !query || text.toLowerCase().includes(query.toLowerCase());
}

// show the user's configured combo for an action, falling back when unbound
export function shortcutHint(id: string, fallback: string): string {
	const combo = getSettings().shortcuts[id]?.[0];
	if (!combo) return fallback;
	return keyComboLabel(combo);
}
