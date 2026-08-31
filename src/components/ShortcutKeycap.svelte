<script lang="ts">
	import { keyComboLabel } from '$lib/shortcuts/index.js';
	import type { ShortcutKey } from '$lib/shortcuts/index.js';

	let {
		combo,
		size = 'md'
	}: {
		combo: ShortcutKey;
		size?: 'sm' | 'md' | 'lg';
	} = $props();

	const capClass = $derived(
		size === 'lg'
			? 'inline-flex min-w-7 items-center justify-center rounded-lg border border-border bg-muted/60 px-2.5 py-1.5 text-[13px] font-medium text-foreground'
			: size === 'sm'
				? 'inline-flex min-w-5 items-center justify-center rounded-md border border-border bg-muted/60 px-1 py-0.5 text-[10px] font-medium text-foreground'
				: 'inline-flex min-w-6 items-center justify-center rounded-md border border-border bg-muted/60 px-1.5 py-0.5 text-[11px] font-medium text-foreground'
	);

	const parts = $derived(keyComboLabel(combo).split(' '));
</script>

<div class="flex items-center gap-1">
	{#each parts as part (part)}
		<kbd class={capClass}>{part}</kbd>
	{/each}
</div>
