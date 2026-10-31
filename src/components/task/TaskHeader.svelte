<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { getSettings } from '$lib/stores/settings';
	import { keyComboLabel } from '$lib/shortcuts/index.js';
	import Icon from '$lib/components/Icon.svelte';

	let {
		pinnedFilter,
		hasFilters,
		filteredCount,
		totalCount,
		viewMode = $bindable<'list' | 'board' | 'calendar'>()
	}: {
		pinnedFilter: boolean;
		hasFilters: boolean;
		filteredCount: number;
		totalCount: number;
		viewMode: 'list' | 'board' | 'calendar';
	} = $props();
</script>

{#snippet shortcut(id: string)}
	{@const combo = getSettings().shortcuts[id]?.[0]}
	{#if combo}
		<span class="ml-1 opacity-50">{keyComboLabel(combo)}</span>
	{/if}
{/snippet}

<header class="flex flex-wrap items-center justify-between gap-2 pb-4 sm:pb-5">
	<div class="flex items-center gap-3">
		<h1 class="text-[18px] font-semibold tracking-tight text-foreground sm:text-[22px]">
			{pinnedFilter ? 'Pinned' : 'Tasks'}
		</h1>
		<div class="flex items-center gap-1.5 text-[12px] text-muted-foreground">
			<span class="size-1.5 rounded-full bg-foreground/30"></span>
			<span>
				{hasFilters || pinnedFilter ? `${filteredCount} of ${totalCount}` : totalCount}
				{totalCount === 1 ? 'task' : 'tasks'}
			</span>
		</div>
	</div>
	<div class="flex items-center gap-0.5 rounded-lg border border-input bg-input/30 p-0.5">
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {viewMode ===
						'list'
							? 'bg-foreground/10 text-foreground'
							: 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'}"
						onclick={() => (viewMode = 'list')}
						aria-label="List view"
					>
						<Icon icon="list" size={15} />
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">List view {@render shortcut('toggle-view')}</Tooltip.Content>
		</Tooltip.Root>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {viewMode ===
						'board'
							? 'bg-foreground/10 text-foreground'
							: 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'}"
						onclick={() => (viewMode = 'board')}
						aria-label="Board view"
					>
						<Icon icon="kanban" size={15} />
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Board view {@render shortcut('toggle-view')}</Tooltip.Content>
		</Tooltip.Root>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {viewMode ===
						'calendar'
							? 'bg-foreground/10 text-foreground'
							: 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'}"
						onclick={() => (viewMode = 'calendar')}
						aria-label="Calendar view"
					>
						<Icon icon="calendar" size={15} />
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom"
				>Calendar view {@render shortcut('toggle-view')}</Tooltip.Content
			>
		</Tooltip.Root>
	</div>
</header>
