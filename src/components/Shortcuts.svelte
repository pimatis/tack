<script lang="ts">
	import { onMount } from 'svelte';
	import { setShortcutRegistry } from '../lib/shortcuts/registry.js';
	import type { ShortcutKey } from '../lib/shortcuts/shortcuts.js';
	import type { ShortcutBehavior } from '../lib/shortcuts/registry.js';
	import { getSettings } from '$lib/stores/settings.js';

	let { children }: { children?: import('svelte').Snippet } = $props();

	function modActive(e: KeyboardEvent, mod?: ShortcutKey['mod']): boolean {
		if (mod === 'meta') return e.metaKey;
		if (mod === 'ctrl') return e.ctrlKey;
		if (mod === 'metaOrCtrl') return e.metaKey || e.ctrlKey;
		return false;
	}

	function comboMatches(k: ShortcutKey, e: KeyboardEvent): boolean {
		// normalize so shift+letter ("C") matches the recorded lowercase key
		const pressedKey = e.key.length === 1 ? e.key.toLowerCase() : e.key;
		if (k.key !== pressedKey) return false;
		if (k.shift && !e.shiftKey) return false;
		if (!k.shift && e.shiftKey) return false;
		if (k.alt && !e.altKey) return false;
		if (!k.alt && e.altKey) return false;
		if (k.mod === undefined) return !e.metaKey && !e.ctrlKey;
		return modActive(e, k.mod);
	}

	let behaviors = $state<ShortcutBehavior[]>([]);

	const api = {
		register(behavior: ShortcutBehavior) {
			behaviors = [...behaviors, behavior];
			return () => {
				behaviors = behaviors.filter((b) => b !== behavior);
			};
		}
	};

	setShortcutRegistry(api);

	function handleKeydown(e: KeyboardEvent) {
		// ignore keys while another app has focus
		if (!document.hasFocus()) return;
		const shortcuts = getSettings().shortcuts;
		for (const [id, keys] of Object.entries(shortcuts)) {
			const combo = keys.find((k) => comboMatches(k, e));
			if (!combo) continue;
			const matches = behaviors.filter((b) => b.id === id && (!b.enabled || b.enabled()));
			if (matches.length === 0) continue;
			e.preventDefault();
			e.stopPropagation();
			for (const b of matches) b.run(e);
			return;
		}
	}

	onMount(() => {
		window.addEventListener('keydown', handleKeydown, true);
		return () => window.removeEventListener('keydown', handleKeydown, true);
	});
</script>

{@render children?.()}
