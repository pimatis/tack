<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import PriorityIcon from './PriorityIcon.svelte';
	import { priorityConfig } from '$lib/task/constants';

	let {
		value,
		onSelect,
		trigger,
		title = 'Set priority',
		align = 'start'
	}: {
		value: number;
		onSelect: (priority: number) => void;
		trigger: Snippet<[Record<string, unknown>]>;
		title?: string;
		align?: 'start' | 'end' | 'center';
	} = $props();
</script>

<Popover.Root>
	<Popover.Trigger>
		{#snippet child({ props })}
			{@render trigger(props)}
		{/snippet}
	</Popover.Trigger>
	<Popover.Content class="w-48 p-1.5" {align}>
		<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
			{title}
		</div>
		{#each [0, 1, 2, 3, 4] as p (p)}
			<Button
				variant="ghost"
				class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
				onclick={() => onSelect(p)}
			>
				<PriorityIcon priority={p} size={14} />
				<span>{priorityConfig[p].label}</span>
				{#if value === p}
					<svg
						class="ml-auto text-muted-foreground"
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d="M21.546 5.111a1.5 1.5 0 0 1 0 2.121L10.303 18.475a1.6 1.6 0 0 1-2.263 0L2.454 12.89a1.5 1.5 0 1 1 2.121-2.121l4.596 4.596L19.424 5.111a1.5 1.5 0 0 1 2.122 0"
						/></svg
					>
				{/if}
			</Button>
		{/each}
	</Popover.Content>
</Popover.Root>
