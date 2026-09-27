<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
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

	let open = $state(false);

	// the trigger opens on right-click too, not just left-click; don't let the
	// row's own context handler open a second menu underneath it
	function openOnRightClick(event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();
		open = true;
	}
</script>

<DropdownMenu.Root bind:open>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			{@render trigger({ ...props, oncontextmenu: openOnRightClick })}
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content class="w-48 p-1.5" {align}>
		<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
			{title}
		</div>
		{#each [0, 1, 2, 3, 4] as p (p)}
			<DropdownMenu.Item class="gap-2.5 py-1.5 text-[13px]" onclick={() => onSelect(p)}>
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
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
