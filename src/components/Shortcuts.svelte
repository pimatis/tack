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

	function anyModPressed(e: KeyboardEvent): boolean {
		return e.metaKey || e.ctrlKey || e.altKey;
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
			const combo = keys.find((k) => {
				if (k.key !== e.key) return false;
				if (k.mod === undefined) return !anyModPressed(e);
				return modActive(e, k.mod);
			});
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
