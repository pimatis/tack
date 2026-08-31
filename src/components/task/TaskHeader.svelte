<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';

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

<header class="flex items-center justify-between pb-5">
	<div class="flex items-center gap-3">
		<h1 class="text-[22px] font-semibold tracking-tight text-foreground">
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
	<div class="flex items-center gap-0.5 rounded-lg border border-border bg-muted/20 p-0.5">
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {viewMode ===
						'list'
							? 'bg-muted text-foreground'
							: 'text-muted-foreground hover:text-foreground'}"
						onclick={() => (viewMode = 'list')}
						aria-label="List view"
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M20 17.5a1.5 1.5 0 0 1 .144 2.993L20 20.5H4a1.5 1.5 0 0 1-.144-2.993L4 17.5zm0-7a1.5 1.5 0 0 1 0 3H4a1.5 1.5 0 0 1 0-3zm0-7a1.5 1.5 0 0 1 0 3H4a1.5 1.5 0 1 1 0-3z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">List view</Tooltip.Content>
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
							? 'bg-muted text-foreground'
							: 'text-muted-foreground hover:text-foreground'}"
						onclick={() => (viewMode = 'board')}
						aria-label="Board view"
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M15.5 20a1.5 1.5 0 0 0 3 0v-1.5H20a1.5 1.5 0 0 0 0-3h-1.5v-7H20a1.5 1.5 0 0 0 0-3h-1.5V4a1.5 1.5 0 0 0-3 0v1.5h-7V4a1.5 1.5 0 1 0-3 0v1.5H4a1.5 1.5 0 1 0 0 3h1.5v7H4a1.5 1.5 0 0 0 0 3h1.5V20a1.5 1.5 0 0 0 3 0v-1.5h7zm-7-4.5h7v-7h-7z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Board view</Tooltip.Content>
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
							? 'bg-muted text-foreground'
							: 'text-muted-foreground hover:text-foreground'}"
						onclick={() => (viewMode = 'calendar')}
						aria-label="Calendar view"
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-1M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Calendar view</Tooltip.Content>
		</Tooltip.Root>
	</div>
</header>
