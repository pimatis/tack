<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { remove } from '$lib/repositories/project.repository';
	import type { Project } from '$lib/types/project';
	import { labelColorMap } from '$lib/types/label';
	import type { TaskStatus } from '$lib/types/task';
	import type { Settings } from '$lib/types/settings';
	import type { SidebarItemConfig, SidebarItemId } from '$lib/types/settings';
	import { setSettings, getSettings } from '$lib/stores/settings';
	import { sortableItem, useDndActive, reorderArray, type DragDropState } from '$lib/dnd';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import PriorityIcon from './PriorityIcon.svelte';
	import StatusIcon from './StatusIcon.svelte';
	import { keyComboLabel } from '$lib/shortcuts/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import UpdateDialog from './UpdateDialog.svelte';
	import { checkForUpdate } from '$lib/updater/update.service';
	import { TaskPageState } from '$lib/task/taskState.svelte';

	let {
		settings = null,
		toggleSidebar = () => {},
		narrow = false,
		mobileOpen = $bindable(false)
	}: {
		settings?: Settings | null;
		toggleSidebar?: () => void;
		narrow?: boolean;
		mobileOpen?: boolean;
	} = $props();

	// on small screens (mobile + tablet) the sidebar is a drawer that starts
	// closed; the desktop collapsed setting is ignored there
	let localMobile = $state(false);
	const isMobile = $derived(narrow || localMobile);
	const collapsed = $derived(isMobile ? !mobileOpen : !!settings?.sidebarCollapsed);

	function handleToggle() {
		if (isMobile) mobileOpen = !mobileOpen;
		else toggleSidebar();
	}

	function closeMobile() {
		if (isMobile) mobileOpen = false;
	}

	// shared task state: the home page owns it, the sidebar only reads it
	const pageState = TaskPageState.get();
	const allTasks = $derived(pageState?.tasks ?? []);
	const projects = $derived(pageState?.projects ?? []);
	const labels = $derived(pageState?.labels ?? []);
	const pinnedCount = $derived(allTasks.filter((t) => t.pinned && !t.deletedAt).length);
	let pinnedActive = $state(false);
	let activeFilter = $state<string | null>(null);

	let updateAvailable = $state(false);
	let updateVersion = $state('');
	let updateDialogOpen = $state(false);

	async function checkForUpdates() {
		try {
			const update = await checkForUpdate();
			if (update) {
				updateAvailable = true;
				updateVersion = update.version;
			}
		} catch {
			// offline or not configured yet; stay quiet
		}
	}

	// sidebar item visibility helpers from settings
	let sidebarItemMap = $derived.by(() => {
		const map = new Map<SidebarItemId, SidebarItemConfig>();
		for (const item of settings?.sidebarItems ?? []) {
			map.set(item.id, item);
		}
		return map;
	});

	function isItemVisible(id: SidebarItemId): boolean {
		return sidebarItemMap.get(id)?.visible ?? true;
	}

	// ordered list of visible reorderable items (between Home and Projects)
	let orderedFilterItems = $derived.by(() => {
		const items = settings?.sidebarItems ?? [];
		return items.filter(
			(item) =>
				item.visible &&
				(item.id === 'pinned' ||
					item.id === 'today' ||
					item.id === 'upcoming' ||
					item.id === 'overdue' ||
					item.id === 'status' ||
					item.id === 'priority')
		);
	});

	// prevent text selection during drag
	$effect(() => {
		useDndActive();
	});

	const statusConfig: Record<TaskStatus, { label: string }> = {
		todo: { label: 'Todo' },
		in_progress: { label: 'In progress' },
		done: { label: 'Done' },
		canceled: { label: 'Canceled' }
	};
	const statusOrder: TaskStatus[] = ['todo', 'in_progress', 'done', 'canceled'];
	const priorityConfig: Record<number, { label: string }> = {
		1: { label: 'Urgent' },
		2: { label: 'High' },
		3: { label: 'Medium' },
		4: { label: 'Low' }
	};

	// computed counts
	let statusCounts = $derived.by(() => {
		const counts: Record<string, number> = { todo: 0, in_progress: 0, done: 0, canceled: 0 };
		for (const t of allTasks) {
			if (t.deletedAt) continue;
			counts[t.status]++;
		}
		return counts;
	});

	let priorityCounts = $derived.by(() => {
		const counts: Record<number, number> = { 0: 0, 1: 0, 2: 0, 3: 0, 4: 0 };
		for (const t of allTasks) {
			if (t.deletedAt) continue;
			counts[t.priority]++;
		}
		return counts;
	});

	let labelCounts = $derived.by(() => {
		const counts = new Map<string, number>();
		for (const t of allTasks) {
			if (t.deletedAt) continue;
			for (const labelId of t.labelIds ?? []) {
				counts.set(labelId, (counts.get(labelId) ?? 0) + 1);
			}
		}
		return counts;
	});

	let todayCount = $derived.by(() => {
		const now = new Date();
		now.setHours(0, 0, 0, 0);
		return allTasks.filter((t) => {
			if (t.deletedAt || t.status === 'done' || t.status === 'canceled') return false;
			if (!t.dueDate) return false;
			return new Date(t.dueDate + 'T00:00:00').getTime() === now.getTime();
		}).length;
	});

	let upcomingCount = $derived.by(() => {
		const now = new Date();
		now.setHours(0, 0, 0, 0);
		return allTasks.filter((t) => {
			if (t.deletedAt || t.status === 'done' || t.status === 'canceled') return false;
			if (!t.dueDate) return false;
			const diffDays = Math.round(
				(new Date(t.dueDate + 'T00:00:00').getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
			);
			return diffDays > 0 && diffDays <= 7;
		}).length;
	});

	let overdueCount = $derived.by(() => {
		const now = new Date();
		now.setHours(0, 0, 0, 0);
		return allTasks.filter((t) => {
			if (t.deletedAt || t.status === 'done' || t.status === 'canceled') return false;
			if (!t.dueDate) return false;
			return new Date(t.dueDate + 'T00:00:00').getTime() < now.getTime();
		}).length;
	});

	let totalTasks = $derived(allTasks.filter((t) => !t.deletedAt).length);
	let doneCount = $derived(allTasks.filter((t) => !t.deletedAt && t.status === 'done').length);
	let doneProgress = $derived(totalTasks > 0 ? Math.round((doneCount / totalTasks) * 100) : 0);

	// navigate home if not already there, then dispatch event
	async function goHomeThenDispatch(eventName: string, detail?: unknown) {
		if (isMobile) mobileOpen = false;
		if (window.location.pathname !== '/') {
			await goto('/');
			await tick();
		}
		if (detail !== undefined) {
			window.dispatchEvent(new CustomEvent(eventName, { detail }));
		} else {
			window.dispatchEvent(new Event(eventName));
		}
	}

	function editProject(project: Project) {
		void goHomeThenDispatch('open-project-edit-dialog', project);
	}

	async function deleteProject(project: Project) {
		try {
			await remove(project.id);
			void goHomeThenDispatch('projects-changed');
		} catch {
			return;
		}
	}

	function dispatchFilter(eventName: string, filterKey: string, detail?: unknown) {
		activeFilter = filterKey;
		void goHomeThenDispatch(eventName, detail);
	}

	function handleSidebarReorder(
		state: DragDropState<SidebarItemConfig>,
		targetItem: SidebarItemConfig
	) {
		const dragged = state.draggedItem;
		if (!dragged || !settings || dragged.id === targetItem.id || !state.dropPosition) return;

		setSettings({
			sidebarItems: reorderArray(settings.sidebarItems, dragged, targetItem, state.dropPosition)
		});
	}

	onMount(() => {
		// tablet and below: drawer mode; layout already forces narrow below 1024px
		const mobileQuery = window.matchMedia('(max-width: 1023px)');
		const applyMobile = () => {
			localMobile = mobileQuery.matches;
			if (!localMobile) mobileOpen = false;
		};
		applyMobile();
		mobileQuery.addEventListener('change', applyMobile);

		const setPinnedActive = () => {
			pinnedActive = true;
			activeFilter = null;
		};
		const setAllInactive = () => {
			pinnedActive = false;
			activeFilter = null;
		};
		const setFilterActive = (event: Event) => {
			pinnedActive = false;
			const detail = (event as CustomEvent).detail;
			if (detail !== undefined) {
				activeFilter = `${event.type}:${detail}`;
			} else {
				activeFilter = event.type;
			}
		};
		window.addEventListener('projects-changed', setAllInactive);
		window.addEventListener('tasks-changed', setAllInactive);
		window.addEventListener('filter-pinned', setPinnedActive);
		window.addEventListener('filter-by-project', setFilterActive);
		window.addEventListener('filter-by-status', setFilterActive);
		window.addEventListener('filter-by-priority', setFilterActive);
		window.addEventListener('filter-by-label', setFilterActive);
		window.addEventListener('filter-today', setFilterActive);
		window.addEventListener('filter-upcoming', setFilterActive);
		const clearActiveState = () => {
			pinnedActive = false;
			activeFilter = null;
		};
		window.addEventListener('filter-overdue', setFilterActive);
		window.addEventListener('clear-filters', clearActiveState);
		// update check is not needed for first paint; let the ui settle first
		const updateTimer = window.setTimeout(() => void checkForUpdates(), 4000);

		const registry = getShortcutRegistry();
		const unregisterToggleSidebar = registry?.register({
			id: 'toggle-sidebar',
			run: () => toggleSidebar()
		});

		return () => {
			window.clearTimeout(updateTimer);
			mobileQuery.removeEventListener('change', applyMobile);
			window.removeEventListener('projects-changed', setAllInactive);
			window.removeEventListener('tasks-changed', setAllInactive);
			window.removeEventListener('filter-pinned', setPinnedActive);
			window.removeEventListener('filter-by-project', setAllInactive);
			window.removeEventListener('filter-by-status', setFilterActive);
			window.removeEventListener('filter-by-priority', setFilterActive);
			window.removeEventListener('filter-by-label', setFilterActive);
			window.removeEventListener('filter-today', setFilterActive);
			window.removeEventListener('filter-upcoming', setFilterActive);
			window.removeEventListener('filter-overdue', setFilterActive);
			window.removeEventListener('clear-filters', clearActiveState);
			unregisterToggleSidebar?.();
		};
	});
</script>

{#snippet shortcut(id: string)}
	{@const combo = getSettings().shortcuts[id]?.[0]}
	{#if combo}
		<span class="ml-1 opacity-50">{keyComboLabel(combo)}</span>
	{/if}
{/snippet}

<!-- mobile drawer backdrop: sibling of the aside so it never covers the
     drawer content itself (clicking an item must not close the drawer) -->
{#if isMobile && mobileOpen}
	<button
		type="button"
		class="fixed inset-0 z-40 bg-black/40"
		aria-label="Close sidebar"
		onclick={closeMobile}
	></button>
{/if}

<aside
	class="flex h-dvh flex-col transition-all duration-200 {collapsed
		? 'w-12'
		: 'w-50'} {isMobile
		? 'fixed left-0 top-0 z-50 bg-sidebar shadow-xl shadow-black/10 max-md:w-60!'
		: ''} {isMobile && !mobileOpen ? '-translate-x-full' : ''}"
>
	<!-- drag region for macOS traffic lights -->
	<div class="h-7 shrink-0" data-tauri-drag-region></div>
	<!-- header: workspace name + quick actions -->
	<div
		class="flex shrink-0 items-center {collapsed
			? 'flex-col justify-center gap-1 px-0 py-2'
			: 'h-12 justify-between px-2.5'}"
	>
		{#if !collapsed}
			<div class="flex items-center gap-2">
				<svg
					width="40"
					height="40"
					viewBox="0 0 1024 1024"
					fill="none"
					aria-label="Tack"
					class="shrink-0 text-sidebar-foreground"
				>
					<path
						d="M433.067 316.588H433.153C450.47 316.652 467.751 316.715 484.915 316.715L529.218 316.646C530.938 316.632 532.71 316.611 534.522 316.59H534.532C547.275 316.443 561.961 316.274 573.892 318.074C593.004 320.959 615.125 328.199 632.202 337.221C661.467 352.602 683.097 378.307 692.77 409.975C700.473 435.194 700.341 461.618 700.21 487.822L700.209 487.865C700.192 491.535 700.174 495.202 700.177 498.86L700.2 571.92L700.22 659.91C700.22 660.647 700.229 662.151 700.241 664.184V664.245C700.31 675.994 700.482 705.161 699.659 706.71C673.795 706.061 643.117 689.4 624.979 671.385C609.689 655.936 598.904 636.608 593.784 615.484C590.13 600.169 590.428 571.748 590.649 550.702C590.697 546.199 590.74 542.032 590.741 538.405L590.761 433.447L502.866 433.335C498.968 433.345 494.833 433.389 490.54 433.435H490.527C467.281 433.681 439.395 433.976 419.351 429.287C398.776 424.682 379.755 414.799 364.16 400.61C345.874 383.672 333.581 360.951 329.264 336.428C329.179 335.946 329.058 335.315 328.916 334.575L328.915 334.573C327.928 329.421 325.931 318.999 328.056 316.727C362.763 316.332 397.99 316.46 433.067 316.588Z"
						fill="currentColor"
					/>
					<path
						d="M558.386 439.245C567.366 439.247 576.347 439.201 585.326 439.105C579.051 452.41 571.317 466.053 564.372 479.137L530.317 543.956L501.76 599.102C494.987 612.344 487.932 627.11 480.132 639.664C474.68 648.43 468.262 656.558 460.999 663.894C431.953 693.287 393.089 707.278 352.295 707.504L323.696 707.476L386.946 588.074L408.027 547.892C420.655 523.345 431.885 499.558 451.609 479.774C482.238 449.052 516.736 439.62 558.386 439.245Z"
						fill="currentColor"
					/>
					<path
						d="M580.123 585.712C581.953 585.515 585.168 585.535 587.692 585.551C588.416 585.556 589.083 585.56 589.644 585.559C586.131 619.27 577.556 651.791 552.886 676.56C535.663 693.659 513.097 704.334 488.953 706.804C481.104 707.607 472.819 707.562 464.779 707.519H464.758C463.333 707.511 461.916 707.504 460.511 707.501L449.458 707.481C454.436 696.89 461.239 686.137 466.112 675.668C491.228 621.711 515.631 590.797 580.123 585.712Z"
						fill="currentColor"
					/>
				</svg>
			</div>
		{/if}
		{#if collapsed && !isMobile}
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
							onclick={handleToggle}
							aria-label="Expand sidebar"
						>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
								/></svg
							>
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="bottom"
					>Expand sidebar {@render shortcut('toggle-sidebar')}</Tooltip.Content
				>
			</Tooltip.Root>
		{/if}
		<div class="flex items-center gap-0.5 {collapsed ? 'flex-col gap-1' : ''}">
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							aria-label="Search"
							onclick={() => void goHomeThenDispatch('open-command-palette')}
							class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
						>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
								/></svg
							>
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="bottom">Search {@render shortcut('command-palette')}</Tooltip.Content
				>
			</Tooltip.Root>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							aria-label="New task"
							onclick={() => void goHomeThenDispatch('open-task-dialog')}
							class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
						>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
								/></svg
							>
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="bottom">New task {@render shortcut('new-task')}</Tooltip.Content>
			</Tooltip.Root>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							aria-label="New project"
							onclick={() => void goHomeThenDispatch('open-project-dialog')}
							class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
						>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
								/></svg
							>
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="bottom"
					>New project {@render shortcut('new-project')}</Tooltip.Content
				>
			</Tooltip.Root>
		</div>
	</div>

	<Separator class="mx-2 my-2 bg-sidebar-border/40" />

	<ScrollArea class="min-h-0 flex-1">
		<!-- home -->
		<div class={collapsed ? 'mb-1 flex justify-center px-0' : 'px-1.5'}>
			<Button
				variant="ghost"
				href="/"
				size={collapsed ? 'icon-sm' : 'default'}
				class={collapsed
					? 'text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'
					: 'w-full justify-start gap-2 px-2 py-1.5 text-[13px] text-sidebar-foreground/90 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}
				onclick={(e) => {
					e.preventDefault();
					void goHomeThenDispatch('clear-filters');
				}}
				aria-label="Home"
			>
				<svg
					class="shrink-0 text-muted-foreground"
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					><path
						fill="currentColor"
						d="M13.2 2.65a2 2 0 0 0-2.4 0l-7 5.25A2 2 0 0 0 3 9.5V19a2 2 0 0 0 2 2h3.9a1.1 1.1 0 0 0 1.1-1.1V15a2 2 0 1 1 4 0v4.9a1.1 1.1 0 0 0 1.1 1.1H19a2 2 0 0 0 2-2V9.5a2 2 0 0 0-.8-1.6z"
					/></svg
				>
				{#if !collapsed}<span>Home</span>{/if}
			</Button>
		</div>

		<!-- dynamic filter items (reorderable via settings) -->
		{#if !collapsed}
			<div class="px-3 pb-1 pt-2 text-[11px] font-medium text-muted-foreground/60">Filters</div>
			{#each orderedFilterItems as item (item.id)}
				{#if item.id === 'pinned'}
					<div
						class="relative px-1.5"
						role="listitem"
						aria-label="Pinned"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<button
							type="button"
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {pinnedActive
								? 'bg-sidebar-accent/70 text-sidebar-foreground'
								: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
							onclick={() => void goHomeThenDispatch('filter-pinned')}
						>
							<svg
								class="shrink-0 text-muted-foreground"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
							>
								<path
									fill="currentColor"
									d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
								/>
							</svg>
							<span>Pinned</span>
							{#if pinnedCount > 0}
								<span
									class="ml-auto text-[11px] tabular-nums text-muted-foreground/50">{pinnedCount}</span
								>
							{/if}
						</button>
					</div>
				{:else if item.id === 'today'}
					<div
						class="relative px-1.5"
						role="listitem"
						aria-label="Today"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<button
							type="button"
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {activeFilter ===
							'filter-today'
								? 'bg-sidebar-accent/70 text-sidebar-foreground'
								: 'text-sidebar-foreground/90 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
							onclick={() => dispatchFilter('filter-today', 'filter-today')}
						>
							<svg
								class="shrink-0 text-muted-foreground"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								><path
									fill="currentColor"
									d="M12 4a9 9 0 1 1 0 18 9 9 0 0 1 0-18m0 4a1 1 0 0 0-.993.883L11 9v3.986a.998.998 0 0 0 .202.617l.09.104 2.106 2.105a1 1 0 0 0 1.498-1.32l-.084-.094L13 12.586V9a1 1 0 0 0-1-1m6-5.394a12.054 12.054 0 0 1 3.272 2.776 1 1 0 0 1-1.544 1.27 10.053 10.053 0 0 0-2.729-2.315 1 1 0 1 1 1.002-1.731Zm-10.634.365A1 1 0 0 1 7 4.337a10.053 10.053 0 0 0-2.729 2.316 1 1 0 1 1-1.544-1.27 12.053 12.053 0 0 1 3.271-2.777 1 1 0 0 1 1.367.365Z"
								/></svg
							>
							<span>Today</span>
							{#if todayCount > 0}
								<span
									class="ml-auto text-[11px] tabular-nums text-muted-foreground/50">{todayCount}</span
								>
							{/if}
						</button>
					</div>
				{:else if item.id === 'upcoming'}
					<div
						class="relative px-1.5"
						role="listitem"
						aria-label="Upcoming"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<button
							type="button"
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {activeFilter ===
							'filter-upcoming'
								? 'bg-sidebar-accent/70 text-sidebar-foreground'
								: 'text-sidebar-foreground/90 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
							onclick={() => dispatchFilter('filter-upcoming', 'filter-upcoming')}
						>
							<svg
								class="shrink-0 text-muted-foreground"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								><path
									fill="currentColor"
									d="M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-3M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
								/></svg
							>
							<span>Upcoming</span>
							{#if upcomingCount > 0}
								<span
									class="ml-auto text-[11px] tabular-nums text-muted-foreground/50">{upcomingCount}</span
								>
							{/if}
						</button>
					</div>
				{:else if item.id === 'overdue'}
					<div
						class="relative px-1.5"
						role="listitem"
						aria-label="Overdue"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<button
							type="button"
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {activeFilter ===
							'filter-overdue'
								? 'bg-sidebar-accent/70 text-sidebar-foreground'
								: 'text-sidebar-foreground/90 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
							onclick={() => dispatchFilter('filter-overdue', 'filter-overdue')}
						>
							<svg
								class="shrink-0 {overdueCount > 0 ? 'text-red-400' : 'text-muted-foreground'}"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								><path
									fill="currentColor"
									d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m0 13a1 1 0 1 0 0 2 1 1 0 0 0 0-2m0-9a1 1 0 0 0-.993.883L11 7v6a1 1 0 0 0 1.993.117L13 13V7a1 1 0 0 0-1-1"
								/></svg
							>
							<span>Overdue</span>
							{#if overdueCount > 0}
								<span
									class="ml-auto text-[11px] tabular-nums text-red-400/80">{overdueCount}</span
								>
							{/if}
						</button>
					</div>
				{:else if item.id === 'status'}
					<div
						class="relative px-1.5"
						role="listitem"
						aria-label="Status"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<details open class="group/status">
							<summary
								class="flex cursor-pointer list-none items-center gap-1 rounded-md px-2 py-1 text-[13px] font-medium text-muted-foreground transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
							>
								<svg
									class="text-muted-foreground transition-transform duration-150 group-open/status:rotate-90"
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									><path
										fill="currentColor"
										d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
									/></svg
								>
								<span>Status</span>
							</summary>
							<div class="mt-0.5 grid gap-px">
								{#each statusOrder as status (status)}
									<button
										type="button"
										class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {activeFilter ===
										'filter-by-status:' + status
											? 'bg-sidebar-accent/70 text-sidebar-foreground'
											: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
										onclick={() =>
											dispatchFilter('filter-by-status', 'filter-by-status:' + status, status)}
									>
										<StatusIcon {status} size={14} />
										<span>{statusConfig[status].label}</span>
										{#if statusCounts[status] > 0}
											<span class="ml-auto shrink-0 text-[11px] text-muted-foreground/60"
												>{statusCounts[status]}</span
											>
										{/if}
									</button>
								{/each}
							</div>
						</details>
					</div>
				{:else if item.id === 'priority'}
					<div
						class="relative px-1.5"
						role="listitem"
						aria-label="Priority"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<details open class="group/priority">
							<summary
								class="flex cursor-pointer list-none items-center gap-1 rounded-md px-2 py-1 text-[13px] font-medium text-muted-foreground transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
							>
								<svg
									class="text-muted-foreground transition-transform duration-150 group-open/priority:rotate-90"
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									><path
										fill="currentColor"
										d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
									/></svg
								>
								<span>Priority</span>
							</summary>
							<div class="mt-0.5 grid gap-px">
								{#each [1, 2, 3, 4] as p (p)}
									<button
										type="button"
										class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {activeFilter ===
										'filter-by-priority:' + p
											? 'bg-sidebar-accent/70 text-sidebar-foreground'
											: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
										onclick={() =>
											dispatchFilter('filter-by-priority', 'filter-by-priority:' + p, p)}
									>
										<PriorityIcon priority={p} size={14} />
										<span>{priorityConfig[p].label}</span>
										{#if priorityCounts[p] > 0}
											<span class="ml-auto shrink-0 text-[11px] text-muted-foreground/60"
												>{priorityCounts[p]}</span
											>
										{/if}
									</button>
								{/each}
							</div>
						</details>
					</div>
				{/if}
			{/each}
		{:else}
			<!-- collapsed: show pinned only -->
			{#if isItemVisible('pinned')}
				<div class="flex justify-center px-0">
					<Button
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground {pinnedActive
							? 'bg-sidebar-accent/70 text-sidebar-foreground'
							: ''}"
						onclick={() => void goHomeThenDispatch('filter-pinned')}
						aria-label="Pinned"
					>
						<svg
							class="shrink-0 text-muted-foreground"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
						>
							<path
								fill="currentColor"
								d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
							/>
						</svg>
					</Button>
				</div>
			{/if}
		{/if}

		<!-- projects -->
		{#if !collapsed}
			<div class="px-1.5">
				<details open class="group/projects">
					<summary
						class="flex cursor-pointer list-none items-center gap-1 rounded-md px-2 py-1 text-[13px] font-medium text-muted-foreground transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
					>
						<svg
							class="text-muted-foreground transition-transform duration-150 group-open/projects:rotate-90"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
							/></svg
						>
						{#if !collapsed}<span>Projects</span>{/if}
						{#if !collapsed}
							<Tooltip.Root>
								<Tooltip.Trigger>
									{#snippet child({ props })}
										<span
											{...props}
											class="ml-auto text-muted-foreground/40 transition-colors hover:text-muted-foreground"
										>
											<svg
												width="13"
												height="13"
												viewBox="0 0 24 24"
												fill="none"
												aria-label="Projects cannot be reordered"
											>
												<path
													fill="currentColor"
													d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m0 14a1 1 0 1 0 0 2 1 1 0 0 0 0-2m0-9.5a3.625 3.625 0 0 0-3.625 3.625 1 1 0 1 0 2 0 1.625 1.625 0 1 1 2.23 1.51c-.676.27-1.605.962-1.605 2.115V14a1 1 0 1 0 2 0c0-.244.05-.366.261-.47l.087-.04A3.626 3.626 0 0 0 12 6.5"
												/>
											</svg>
										</span>
									{/snippet}
								</Tooltip.Trigger>
								<Tooltip.Content side="right" class="max-w-[220px]">
									Projects are ordered automatically and can't be dragged. Use the toolbar above to
									create or manage them.
								</Tooltip.Content>
							</Tooltip.Root>
						{/if}
					</summary>
					<div class="mt-0.5 grid gap-px">
						{#each projects as project (project.id)}
							<ContextMenu.Root>
								<ContextMenu.Trigger
									class="flex w-full min-w-0 items-center rounded-md transition-colors hover:bg-sidebar-accent"
								>
									<Button
										variant="ghost"
										class="flex h-auto min-w-0 flex-1 items-center gap-2 rounded-none px-2 py-1.5 text-left text-[13px] text-sidebar-foreground/90"
										onclick={() => void goHomeThenDispatch('filter-by-project', project.id)}
									>
										<svg
											class="shrink-0 text-muted-foreground"
											width="14"
											height="14"
											viewBox="0 0 24 24"
											fill="none"
											><path
												fill="currentColor"
												d="M9.686 2.512a1.5 1.5 0 0 1 1.303 1.674L10.637 7h3.976l.399-3.186a1.5 1.5 0 0 1 2.977.372L17.637 7H20a1.5 1.5 0 0 1 0 3h-2.738l-.5 4H19.5a1.5 1.5 0 0 1 0 3h-3.113l-.398 3.186a1.5 1.5 0 0 1-2.977-.372L13.363 17H9.388l-.398 3.186a1.5 1.5 0 1 1-2.977-.372L6.363 17H4.5a1.5 1.5 0 1 1 0-3h2.238l.5-4H5a1.5 1.5 0 1 1 0-3h2.613l.399-3.186A1.5 1.5 0 0 1 9.686 2.51ZM13.74 14l.5-4h-3.977l-.5 4z"
											/></svg
										>
										<span class="truncate">{project.name}</span>
										<span class="ml-auto shrink-0 text-[11px] text-muted-foreground/60"
											>{project.prefix}</span
										>
									</Button>
								</ContextMenu.Trigger>
								<ContextMenu.Content>
									<ContextMenu.Item onclick={() => editProject(project)}>
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
											><path
												fill="currentColor"
												d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
											/></svg
										>
										Edit project
									</ContextMenu.Item>
									<ContextMenu.Item
										variant="destructive"
										onclick={() => void deleteProject(project)}
									>
										<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
											><path
												fill="currentColor"
												d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
											/></svg
										>
										Delete project
									</ContextMenu.Item>
								</ContextMenu.Content>
							</ContextMenu.Root>
						{/each}
					</div>
				</details>

				<!-- labels -->
				{#if labels.length > 0}
					<details open class="group/labels mt-0.5">
						<summary
							class="flex cursor-pointer list-none items-center gap-1 rounded-md px-2 py-1 text-[13px] font-medium text-muted-foreground transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
						>
							<svg
								class="text-muted-foreground transition-transform duration-150 group-open/labels:rotate-90"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								><path
									fill="currentColor"
									d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
								/></svg
							>
							<span>Labels</span>
						</summary>
						<div class="mt-0.5 grid gap-px">
							{#each labels as label (label.id)}
								<button
									type="button"
									class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {activeFilter ===
									'filter-by-label:' + label.id
										? 'bg-sidebar-accent/70 text-sidebar-foreground'
										: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
									onclick={() =>
										dispatchFilter('filter-by-label', 'filter-by-label:' + label.id, label.id)}
								>
									<span class="h-2.5 w-2.5 shrink-0 rounded-full {labelColorMap[label.color].dot}"
									></span>
									<span class="truncate">{label.name}</span>
									{#if (labelCounts.get(label.id) ?? 0) > 0}
										<span class="ml-auto shrink-0 text-[11px] text-muted-foreground/60"
											>{labelCounts.get(label.id)}</span
										>
									{/if}
								</button>
							{/each}
						</div>
					</details>
				{/if}
			</div>
		{/if}
		<div class="h-2 shrink-0"></div>
	</ScrollArea>

	<!-- stats footer -->
	{#if !collapsed && totalTasks > 0 && isItemVisible('quickStats')}
		<div class="shrink-0 border-t border-sidebar-border/40 px-3 py-2">
			<div class="mb-1.5 flex items-center justify-between text-[11px] text-muted-foreground">
				<span>{totalTasks} tasks</span>
				<span>{doneProgress}% done</span>
			</div>
			<div class="h-1 overflow-hidden rounded-full bg-sidebar-accent">
				<div
					class="h-full rounded-full bg-green-500/70 transition-all duration-300"
					style="width: {doneProgress}%"
				></div>
			</div>
		</div>
	{/if}

	<!-- trash + settings -->
	<div
		class="shrink-0 border-t border-sidebar-border/40 {collapsed
			? 'px-0'
			: 'px-2'} py-1.5 {collapsed ? 'flex flex-col items-center gap-1' : ''}"
	>
		{#if updateAvailable && !collapsed}
			<button
				type="button"
				onclick={() => (updateDialogOpen = true)}
				class="mb-1 flex w-full items-center gap-2 rounded-md border border-primary/20 bg-primary/10 px-2 py-1.5 text-left text-[12px] text-sidebar-foreground/90 transition-colors hover:bg-primary/15"
			>
				<svg class="shrink-0 text-primary" width="14" height="14" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M20 14.5a1.5 1.5 0 0 1 1.5 1.5v4a2.5 2.5 0 0 1-2.5 2.5H5A2.5 2.5 0 0 1 2.5 20v-4a1.5 1.5 0 0 1 3 0v3.5h13V16a1.5 1.5 0 0 1 1.5-1.5m-8-13A1.5 1.5 0 0 1 13.5 3v9.036l1.682-1.682a1.5 1.5 0 0 1 2.121 2.12l-4.066 4.067a1.75 1.75 0 0 1-2.474 0l-4.066-4.066a1.5 1.5 0 0 1 2.121-2.121l1.682 1.682V3A1.5 1.5 0 0 1 12 1.5"
					/></svg
				>
				<span class="truncate">Update to {updateVersion} is available</span>
			</button>
		{/if}
		<Button
			variant="ghost"
			href="/trash"
			size={collapsed ? 'icon-sm' : 'default'}
			class={collapsed
				? 'text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'
				: 'w-full justify-start gap-2 px-2 py-1.5 text-[13px] text-sidebar-foreground/90 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}
			aria-label="Trash"
			onclick={() => closeMobile()}
		>
			<svg
				class="shrink-0 text-muted-foreground"
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				><path
					fill="currentColor"
					d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
				/></svg
			>
			{#if !collapsed}<span>Trash</span>{/if}
		</Button>
		<Button
			variant="ghost"
			href="/settings"
			size={collapsed ? 'icon-sm' : 'default'}
			class={collapsed
				? 'text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'
				: 'w-full justify-start gap-2 px-2 py-1.5 text-[13px] text-sidebar-foreground/90 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}
			aria-label="Settings"
			onclick={() => closeMobile()}
		>
			<svg
				class="shrink-0 text-muted-foreground"
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				><path
					fill="currentColor"
					d="M9.965 2.809a1.511 1.511 0 0 0-1.401-.203 9.99 9.99 0 0 0-2.982 1.725 1.51 1.51 0 0 0-.524 1.313c.075.753-.058 1.48-.42 2.106-.361.627-.925 1.106-1.615 1.417a1.511 1.511 0 0 0-.875 1.113 10.059 10.059 0 0 0 0 3.44c.093.537.46.926.875 1.114.69.31 1.254.79 1.616 1.416.361.627.494 1.353.419 2.106-.045.452.107.964.524 1.313a9.989 9.989 0 0 0 2.982 1.725 1.51 1.51 0 0 0 1.4-.203c.615-.442 1.312-.691 2.036-.691s1.42.249 2.035.691c.37.266.89.39 1.401.203a9.99 9.99 0 0 0 2.982-1.725c.417-.349.57-.86.524-1.313-.075-.753.057-1.48.42-2.106.361-.627.925-1.105 1.615-1.416.414-.187.782-.577.875-1.114a10.062 10.062 0 0 0 0-3.44 1.511 1.511 0 0 0-.875-1.113c-.69-.311-1.254-.79-1.616-1.417-.362-.626-.494-1.353-.419-2.106a1.511 1.511 0 0 0-.524-1.313 9.99 9.99 0 0 0-2.982-1.725 1.511 1.511 0 0 0-1.4.203C13.42 3.25 12.723 3.5 12 3.5s-1.42-.249-2.035-.691M9 12a3 3 0 1 1 6 0 3 3 0 0 1-6 0"
				/></svg
			>
			{#if !collapsed}<span>Settings</span>{/if}
		</Button>
	</div>
</aside>

<UpdateDialog bind:open={updateDialogOpen} />
