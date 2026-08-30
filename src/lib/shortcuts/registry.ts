import { getContext, setContext } from 'svelte';

type ShortcutCallback = (e: KeyboardEvent) => void;

export type ShortcutBehavior = {
	id: string;
	enabled?: () => boolean;
	run: ShortcutCallback;
};

export type ShortcutRegistryApi = {
	register(behavior: ShortcutBehavior): () => void;
};

const CONTEXT_KEY = Symbol('tack-shortcuts');

export function getShortcutRegistry(): ShortcutRegistryApi {
	return getContext(CONTEXT_KEY);
}

export function setShortcutRegistry(api: ShortcutRegistryApi) {
	setContext(CONTEXT_KEY, api);
}
