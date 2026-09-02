<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { sortableItem, useDndActive, reorderArray, type DragDropState } from '$lib/dnd';
	import type { Settings, SidebarItemConfig, SidebarItemId } from '$lib/types/settings';
	import { defaultSidebarItems } from '$lib/types/settings';

	let {
		settings,
		update
	}: {
		settings: Settings;
		update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
	} = $props();

	// prevent text selection during drag
	$effect(() => {
		useDndActive();
	});

	const sidebarItemLabels: Record<SidebarItemId, string> = {
		pinned: 'Pinned',
		today: 'Today',
		upcoming: 'Upcoming',
		overdue: 'Overdue',
		status: 'Status',
		priority: 'Priority',
		quickStats: 'Quick stats'
	};

	const sidebarItemDescriptions: Record<SidebarItemId, string> = {
		pinned: 'Show pinned tasks filter',
		today: 'Show tasks due today',
		upcoming: 'Show tasks due in the next 7 days',
		overdue: 'Show tasks past their due date',
		status: 'Show status filter section (Todo, In progress, Done, Canceled)',
		priority: 'Show priority filter section (Urgent, High, Medium, Low)',
		quickStats: 'Show task count and progress bar at sidebar bottom'
	};

	// reorderable items exclude quickStats (fixed at footer)
	let reorderableItems = $derived(settings.sidebarItems.filter((item) => item.id !== 'quickStats'));

	let quickStatsConfig = $derived(settings.sidebarItems.find((item) => item.id === 'quickStats'));

	function toggleSidebarItem(id: SidebarItemId, visible: boolean) {
		const updated = settings.sidebarItems.map((item) =>
			item.id === id ? { ...item, visible } : item
		);
		update('sidebarItems', updated);
	}

	function handleSidebarDrop(
		state: DragDropState<SidebarItemConfig>,
		targetItem: SidebarItemConfig
	) {
		const dragged = state.draggedItem;
		if (!dragged || dragged.id === targetItem.id || !state.dropPosition) return;

		update(
			'sidebarItems',
			reorderArray(settings.sidebarItems, dragged, targetItem, state.dropPosition)
		);
	}

	function resetSidebarItems() {
		update('sidebarItems', [...defaultSidebarItems]);
	}
</script>

<!-- reorderable items -->
<div>
	<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
		<div>
			<p class="text-[13px] font-medium">Sidebar items</p>
			<p class="text-xs text-muted-foreground">Drag to reorder, toggle to show or hide</p>
		</div>
		<Button
			variant="ghost"
			size="sm"
			class="text-xs text-muted-foreground"
			onclick={resetSidebarItems}
		>
			Reset
		</Button>
	</div>
	<div class="space-y-1.5">
		{#each reorderableItems as item (item.id)}
			<div
				class="group/sidebar-row relative flex cursor-grab items-center gap-3 rounded-lg border border-border/50 bg-muted/30 px-3 py-2.5 transition-all hover:border-border active:cursor-grabbing"
				role="listitem"
				aria-label={sidebarItemLabels[item.id]}
				use:sortableItem={{
					dragData: item,
					container: 'sidebar-items',
					onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarDrop(state, item)
				}}
			>
				<!-- drag handle -->
				<svg
					class="shrink-0 text-muted-foreground/40 transition-colors group-hover/sidebar-row:text-muted-foreground"
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
				>
					<circle cx="9" cy="6" r="1.5" fill="currentColor" />
					<circle cx="15" cy="6" r="1.5" fill="currentColor" />
					<circle cx="9" cy="12" r="1.5" fill="currentColor" />
					<circle cx="15" cy="12" r="1.5" fill="currentColor" />
					<circle cx="9" cy="18" r="1.5" fill="currentColor" />
					<circle cx="15" cy="18" r="1.5" fill="currentColor" />
				</svg>
				<!-- item info -->
				<div class="min-w-0 flex-1">
					<p class="text-[13px] font-medium">{sidebarItemLabels[item.id]}</p>
					<p class="truncate text-xs text-muted-foreground">{sidebarItemDescriptions[item.id]}</p>
				</div>
				<!-- toggle -->
				<Switch checked={item.visible} onCheckedChange={(v) => toggleSidebarItem(item.id, v)} />
			</div>
		{/each}
	</div>
</div>

<Separator />

<!-- quick stats (fixed position, toggle only) -->
{#if quickStatsConfig}
	<div
		class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border/50 bg-muted/30 px-3 py-2.5"
	>
		<div class="min-w-0 flex-1">
			<p class="text-[13px] font-medium">{sidebarItemLabels.quickStats}</p>
			<p class="truncate text-xs text-muted-foreground">{sidebarItemDescriptions.quickStats}</p>
		</div>
		<Switch
			checked={quickStatsConfig.visible}
			onCheckedChange={(v) => toggleSidebarItem('quickStats', v)}
		/>
	</div>
{/if}

<!-- fixed items info -->
<div class="rounded-lg bg-muted/20 p-3">
	<p class="text-xs text-muted-foreground">
		Home, projects, labels, trash and settings are always visible
	</p>
</div>
