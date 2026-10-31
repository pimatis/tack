<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import { statusConfig, statusOrder } from '$lib/task/constants';
	import type { TaskStatus } from '$lib/types/task';
	import Icon from '$lib/components/Icon.svelte';

	let {
		value,
		onSelect,
		trigger,
		title = 'Change status',
		align = 'start'
	}: {
		value: TaskStatus;
		onSelect: (status: TaskStatus) => void;
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
		{#each statusOrder as s (s)}
			<DropdownMenu.Item class="gap-2.5 py-1.5 text-[13px]" onclick={() => onSelect(s)}>
				<StatusIcon status={s} size={14} />
				<span>{statusConfig[s].label}</span>
				{#if value === s}
					<Icon icon="check" size={14} class="ml-auto text-muted-foreground" />
				{/if}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
