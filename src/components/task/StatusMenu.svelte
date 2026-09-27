<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import { statusConfig, statusOrder } from '$lib/task/constants';
	import type { TaskStatus } from '$lib/types/task';

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
