<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import PriorityIcon from './PriorityIcon.svelte';
	import { priorityConfig } from '$lib/task/constants';
	import Icon from '$lib/components/Icon.svelte';

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
					<Icon icon="check" size={14} class="ml-auto text-muted-foreground" />
				{/if}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
