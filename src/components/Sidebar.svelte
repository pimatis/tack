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
	import PriorityIcon from './task/PriorityIcon.svelte';
	import StatusIcon from './task/StatusIcon.svelte';
	import { keyComboLabel } from '$lib/shortcuts/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import { checkForUpdate } from '$lib/updater/update.service';
	import { TaskPageState } from '$lib/task/taskState.svelte';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import Icon from '$lib/components/Icon.svelte';
	import type { IconName } from '$lib/icons';

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

	// persist collapsed sidebar sections across page reloads
	type CollapsibleSectionId = 'status' | 'priority' | 'projects' | 'labels';
	const COLLAPSED_SECTIONS_KEY = 'tack-collapsed-sidebar-sections';
	const collapsibleSectionIds: CollapsibleSectionId[] = [
		'status',
		'priority',
		'projects',
		'labels'
	];

	function loadCollapsedSections(): Set<CollapsibleSectionId> {
		try {
			const raw = localStorage.getItem(COLLAPSED_SECTIONS_KEY);
			if (!raw) return new Set();
			const arr: unknown = JSON.parse(raw);
			if (!Array.isArray(arr)) return new Set();
			return new Set(
				arr.filter((s): s is CollapsibleSectionId =>
					collapsibleSectionIds.includes(s as CollapsibleSectionId)
				)
			);
		} catch {
			// localStorage unavailable (ssr) or corrupt - start fresh
			return new Set();
		}
	}

	let collapsedSections = $state<Set<CollapsibleSectionId>>(loadCollapsedSections());

	function setSectionOpen(id: CollapsibleSectionId, open: boolean) {
		const next = new Set(collapsedSections);
		if (open) next.delete(id);
		else next.add(id);
		if (next.size === collapsedSections.size) return;
		collapsedSections = next;
		try {
			localStorage.setItem(COLLAPSED_SECTIONS_KEY, JSON.stringify([...next]));
		} catch {
			// ignore quota errors
		}
	}

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

	// shared chrome classes keep every nav row and group header visually identical
	const navItemClass =
		'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors';
	const navIdleClass =
		'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground';
	const navActiveClass = 'bg-sidebar-accent/70 text-sidebar-foreground';
	const groupTriggerClass =
		'flex w-full cursor-pointer list-none items-center gap-1.5 rounded-md px-2 py-1 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground';
	const groupContentClass = 'mt-0.5 ml-3 grid gap-px pl-2';
	// tree-item marks a row inside a collapsible group; the scoped styles
	// below draw the vertical rail + connector stub onto it
	const treeItemClass = 'tree-item';
	const countClass = 'ml-auto shrink-0 text-[11px] text-muted-foreground/60 tabular-nums';
	const countDangerClass = 'ml-auto shrink-0 text-[11px] text-red-400/80 tabular-nums';
	const iconBtnClass =
		'text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground';

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

	// sidebar list vertical scroll fade: mask top/bottom edges only when scrollable
	let listViewport = $state<HTMLElement | null>(null);
	let listScrollTop = $state(0);
	let listScrollMax = $state(0);

	function updateListScroll() {
		if (!listViewport) return;
		listScrollTop = listViewport.scrollTop;
		listScrollMax = listViewport.scrollHeight - listViewport.clientHeight;
	}

	$effect(() => {
		const el = listViewport;
		if (!el) return;
		updateListScroll();
		el.addEventListener('scroll', updateListScroll, { passive: true });
		const ro = new ResizeObserver(updateListScroll);
		ro.observe(el);
		if (el.firstElementChild) ro.observe(el.firstElementChild);
		return () => {
			el.removeEventListener('scroll', updateListScroll);
			ro.disconnect();
		};
	});

	const listMaskStyle = $derived.by(() => {
		const fadeTop = listScrollTop > 1;
		const fadeBottom = listScrollTop < listScrollMax - 1;
		if (!fadeTop && !fadeBottom) return '';
		const top = fadeTop ? 'transparent 0' : 'black 0';
		const bottom = fadeBottom ? 'transparent 100%' : 'black 100%';
		const m = `linear-gradient(to bottom, ${top}, black 20px, black calc(100% - 20px), ${bottom})`;
		return `mask-image:${m};-webkit-mask-image:${m}`;
	});

	$effect(() => {
		const el = listViewport;
		if (!el) return;
		el.style.maskImage = '';
		el.style.webkitMaskImage = '';
		if (listMaskStyle) {
			const [mi, wi] = listMaskStyle.split(';');
			el.style.maskImage = mi.slice('mask-image:'.length);
			el.style.webkitMaskImage = wi.slice('-webkit-mask-image:'.length);
		}
	});

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

	// right-click on a project opens the dropdown at the pointer
	let projectMenuOpen = $state(false);
	let projectMenuX = $state(0);
	let projectMenuY = $state(0);
	let projectMenuTarget = $state<Project | null>(null);

	function openProjectMenu(event: MouseEvent, project: Project) {
		event.preventDefault();
		projectMenuTarget = project;
		projectMenuX = event.clientX;
		projectMenuY = event.clientY;
		projectMenuOpen = true;
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

<!-- one row shape for every flat filter: icon + label + trailing count -->
{#snippet navRow(
	icon: IconName,
	text: string,
	count: number,
	active: boolean,
	onclick: () => void,
	danger = false
)}
	<button
		type="button"
		class="{navItemClass} {active ? navActiveClass : navIdleClass}"
		{onclick}
		aria-pressed={active}
	>
		<Icon
			{icon}
			size={15}
			class="shrink-0 {danger && count > 0 ? 'text-red-400' : 'text-muted-foreground'}"
		/>
		<span class="truncate">{text}</span>
		{#if count > 0}
			<span class={danger ? countDangerClass : countClass}>{count}</span>
		{/if}
	</button>
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
	class="flex h-dvh shrink-0 flex-col transition-all duration-200 {collapsed
		? 'w-12 pl-2'
		: 'w-56'} {collapsed ? 'overflow-hidden' : ''} {isMobile
		? 'fixed top-0 left-0 z-50 bg-sidebar shadow-xl shadow-black/10 max-md:w-60!'
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
							class={iconBtnClass}
							onclick={handleToggle}
							aria-label="Expand sidebar"
						>
							<Icon icon="arrow-right" size={16} />
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
							onclick={() => {
								// keep the active tab; the palette routes to the right search itself
								if (isMobile) mobileOpen = false;
								window.dispatchEvent(new Event('open-command-palette'));
							}}
							class={iconBtnClass}
						>
							<Icon icon="search" size={16} />
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
							onclick={() => {
								if (isMobile) mobileOpen = false;
								void goHomeThenDispatch('open-task-dialog');
							}}
							class={iconBtnClass}
						>
							<Icon icon="compose" size={16} />
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="bottom"
					>New task
					{@render shortcut('new-task')}</Tooltip.Content
				>
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
							class={iconBtnClass}
						>
							<Icon icon="add" size={16} />
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

	<ScrollArea class="min-h-0 flex-1" bind:viewportRef={listViewport}>
		<!-- home -->
		<div class={collapsed ? 'mb-1 flex justify-center px-0' : 'px-1.5'}>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							href="/"
							size={collapsed ? 'icon-sm' : 'default'}
							class={collapsed
								? iconBtnClass
								: 'w-full justify-start gap-2 px-2 py-1.5 text-[13px] ' + navIdleClass}
							onclick={(e) => {
								e.preventDefault();
								void goHomeThenDispatch('clear-filters');
							}}
							aria-label="Home"
						>
							<Icon icon="home" size={15} class="shrink-0 text-muted-foreground" />
							{#if !collapsed}<span>Home</span>{/if}
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				{#if collapsed}<Tooltip.Content side="right">Home</Tooltip.Content>{/if}
			</Tooltip.Root>
		</div>

		<!-- dynamic filter items (reorderable via settings) -->
		{#if !collapsed}
			<div class="px-3 pt-2 pb-1 text-[12px] font-medium text-muted-foreground">Filters</div>
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
						{@render navRow(
							'pin',
							'Pinned',
							pinnedCount,
							pinnedActive,
							() => void goHomeThenDispatch('filter-pinned')
						)}
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
						{@render navRow('clock', 'Today', todayCount, activeFilter === 'filter-today', () =>
							dispatchFilter('filter-today', 'filter-today')
						)}
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
						{@render navRow(
							'calendar',
							'Upcoming',
							upcomingCount,
							activeFilter === 'filter-upcoming',
							() => dispatchFilter('filter-upcoming', 'filter-upcoming')
						)}
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
						{@render navRow(
							'alert-triangle',
							'Overdue',
							overdueCount,
							activeFilter === 'filter-overdue',
							() => dispatchFilter('filter-overdue', 'filter-overdue'),
							true
						)}
					</div>
				{:else if item.id === 'status'}
					<div
						class="relative px-1.5 pt-1"
						role="listitem"
						aria-label="Status"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<Collapsible.Root
							open={!collapsedSections.has('status')}
							onOpenChange={(open) => setSectionOpen('status', open)}
							class="group/status"
						>
							<Collapsible.Trigger class={groupTriggerClass}>
								<Icon
									icon="arrow-right"
									size={12}
									class="shrink-0 text-muted-foreground transition-transform duration-150 group-data-[state=open]/status:rotate-90"
								/>
								<span>Status</span>
							</Collapsible.Trigger>
							<Collapsible.Content class={groupContentClass}>
								{#each statusOrder as status (status)}
									<button
										type="button"
										aria-pressed={activeFilter === 'filter-by-status:' + status}
										class="{treeItemClass} {navItemClass} {activeFilter ===
										'filter-by-status:' + status
											? navActiveClass
											: navIdleClass}"
										onclick={() =>
											dispatchFilter('filter-by-status', 'filter-by-status:' + status, status)}
									>
										<StatusIcon {status} size={14} />
										<span>{statusConfig[status].label}</span>
										{#if statusCounts[status] > 0}
											<span class={countClass}>{statusCounts[status]}</span>
										{/if}
									</button>
								{/each}
							</Collapsible.Content>
						</Collapsible.Root>
					</div>
				{:else if item.id === 'priority'}
					<div
						class="relative px-1.5 pt-1"
						role="listitem"
						aria-label="Priority"
						use:sortableItem={{
							dragData: item,
							container: 'sidebar-filters',
							onDrop: (state: DragDropState<SidebarItemConfig>) => handleSidebarReorder(state, item)
						}}
					>
						<Collapsible.Root
							open={!collapsedSections.has('priority')}
							onOpenChange={(open) => setSectionOpen('priority', open)}
							class="group/priority"
						>
							<Collapsible.Trigger class={groupTriggerClass}>
								<Icon
									icon="arrow-right"
									size={12}
									class="shrink-0 text-muted-foreground transition-transform duration-150 group-data-[state=open]/priority:rotate-90"
								/>
								<span>Priority</span>
							</Collapsible.Trigger>
							<Collapsible.Content class={groupContentClass}>
								{#each [1, 2, 3, 4] as p (p)}
									<button
										type="button"
										aria-pressed={activeFilter === 'filter-by-priority:' + p}
										class="{treeItemClass} {navItemClass} {activeFilter ===
										'filter-by-priority:' + p
											? navActiveClass
											: navIdleClass}"
										onclick={() =>
											dispatchFilter('filter-by-priority', 'filter-by-priority:' + p, p)}
									>
										<PriorityIcon priority={p} size={14} />
										<span>{priorityConfig[p].label}</span>
										{#if priorityCounts[p] > 0}
											<span class={countClass}>{priorityCounts[p]}</span>
										{/if}
									</button>
								{/each}
							</Collapsible.Content>
						</Collapsible.Root>
					</div>
				{/if}
			{/each}
		{:else}
			<!-- collapsed: show pinned only -->
			{#if isItemVisible('pinned')}
				<div class="flex justify-center px-0">
					<Tooltip.Root>
						<Tooltip.Trigger>
							{#snippet child({ props })}
								<Button
									{...props}
									variant="ghost"
									size="icon-sm"
									class="{iconBtnClass} {pinnedActive ? navActiveClass : ''}"
									onclick={() => void goHomeThenDispatch('filter-pinned')}
									aria-label="Pinned"
								>
									<Icon icon="pin" size={15} class="shrink-0 text-muted-foreground" />
								</Button>
							{/snippet}
						</Tooltip.Trigger>
						<Tooltip.Content side="right">Pinned</Tooltip.Content>
					</Tooltip.Root>
				</div>
			{/if}
		{/if}

		<!-- projects -->
		{#if !collapsed}
			<div class="px-1.5 pt-1">
				<Collapsible.Root
					open={!collapsedSections.has('projects')}
					onOpenChange={(open) => setSectionOpen('projects', open)}
					class="group/projects"
				>
					<div class="flex items-center gap-1 pr-1">
						<Collapsible.Trigger class="{groupTriggerClass} flex-1">
							<Icon
								icon="arrow-right"
								size={12}
								class="shrink-0 text-muted-foreground transition-transform duration-150 group-data-[state=open]/projects:rotate-90"
							/>
							<span>Projects</span>
						</Collapsible.Trigger>
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<button
										{...props}
										type="button"
										aria-label="New project"
										class="rounded-md p-1 text-muted-foreground/50 transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
										onclick={(e) => {
											e.stopPropagation();
											void goHomeThenDispatch('open-project-dialog');
										}}
									>
										<Icon icon="add" size={13} />
									</button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="right"
								>New project {@render shortcut('new-project')}</Tooltip.Content
							>
						</Tooltip.Root>
					</div>
					<Collapsible.Content class={groupContentClass}>
						{#if projects.length === 0}
							<div class="px-2 py-1.5 text-[12px] text-muted-foreground/60">No projects yet</div>
						{/if}
						{#each projects as project (project.id)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								oncontextmenu={(e) => openProjectMenu(e, project)}
								class="{treeItemClass} flex w-full min-w-0 items-center rounded-md transition-colors hover:bg-sidebar-accent/50 {activeFilter ===
								'filter-by-project:' + project.id
									? 'bg-sidebar-accent/70'
									: ''}"
							>
								<Button
									variant="ghost"
									class="flex h-auto min-w-0 flex-1 items-center gap-2 rounded-none px-2 py-1.5 text-left text-[13px] text-sidebar-foreground/80 hover:text-sidebar-foreground"
									onclick={() => void goHomeThenDispatch('filter-by-project', project.id)}
								>
									<Icon icon="hashtag" size={15} class="shrink-0 text-muted-foreground" />
									<span class="truncate">{project.name}</span>
									<span class={countClass}>{project.prefix}</span>
								</Button>
							</div>
						{/each}
						<DropdownMenu.Root bind:open={projectMenuOpen}>
							<DropdownMenu.Trigger
								class="h-0 w-0 outline-none"
								style="position: fixed; left: {projectMenuX}px; top: {projectMenuY}px"
							/>
							<DropdownMenu.Content class="w-44">
								{#if projectMenuTarget}
									<DropdownMenu.Item onclick={() => editProject(projectMenuTarget!)}>
										<Icon icon="pencil" size={15} />
										Edit project
									</DropdownMenu.Item>
									<DropdownMenu.Item
										variant="destructive"
										onclick={() => void deleteProject(projectMenuTarget!)}
									>
										<Icon icon="trash" size={15} />
										Delete project
									</DropdownMenu.Item>
								{/if}
							</DropdownMenu.Content>
						</DropdownMenu.Root>
					</Collapsible.Content>
				</Collapsible.Root>

				<!-- labels -->
				{#if labels.length > 0}
					<Collapsible.Root
						open={!collapsedSections.has('labels')}
						onOpenChange={(open) => setSectionOpen('labels', open)}
						class="group/labels mt-1"
					>
						<Collapsible.Trigger class={groupTriggerClass}>
							<Icon
								icon="arrow-right"
								size={12}
								class="shrink-0 text-muted-foreground transition-transform duration-150 group-data-[state=open]/labels:rotate-90"
							/>
							<span>Labels</span>
						</Collapsible.Trigger>
						<Collapsible.Content class={groupContentClass}>
							{#each labels as label (label.id)}
								<button
									type="button"
									aria-pressed={activeFilter === 'filter-by-label:' + label.id}
									class="{treeItemClass} {navItemClass} {activeFilter ===
									'filter-by-label:' + label.id
										? navActiveClass
										: navIdleClass}"
									onclick={() =>
										dispatchFilter('filter-by-label', 'filter-by-label:' + label.id, label.id)}
								>
									<span class="h-2.5 w-2.5 shrink-0 rounded-full {labelColorMap[label.color].dot}"
									></span>
									<span class="truncate">{label.name}</span>
									{#if (labelCounts.get(label.id) ?? 0) > 0}
										<span class={countClass}>{labelCounts.get(label.id)}</span>
									{/if}
								</button>
							{/each}
						</Collapsible.Content>
					</Collapsible.Root>
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
				<Icon icon="download" size={15} class="shrink-0 text-primary" />
				<span class="truncate">Update to {updateVersion} is available</span>
			</button>
		{/if}
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						href="/trash"
						size={collapsed ? 'icon-sm' : 'default'}
						class={collapsed
							? iconBtnClass
							: 'w-full justify-start gap-2 px-2 py-1.5 text-[13px] ' + navIdleClass}
						aria-label="Trash"
						onclick={() => {
							// keep the active tab so the trash opens on the same tab's items
							closeMobile();
						}}
					>
						<Icon icon="trash" size={15} class="shrink-0 text-muted-foreground" />
						{#if !collapsed}<span>Trash</span>{/if}
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			{#if collapsed}<Tooltip.Content side="right">Trash</Tooltip.Content>{/if}
		</Tooltip.Root>
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						href="/settings"
						size={collapsed ? 'icon-sm' : 'default'}
						class={collapsed
							? iconBtnClass
							: 'w-full justify-start gap-2 px-2 py-1.5 text-[13px] ' + navIdleClass}
						aria-label="Settings"
						onclick={() => {
							// keep the active tab; settings renders on its own route
							closeMobile();
						}}
					>
						<Icon icon="settings" size={15} class="shrink-0 text-muted-foreground" />
						{#if !collapsed}<span>Settings</span>{/if}
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			{#if collapsed}<Tooltip.Content side="right">Settings</Tooltip.Content>{/if}
		</Tooltip.Root>
	</div>
</aside>

{#if updateDialogOpen}
	{#await import('./UpdateDialog.svelte') then { default: UpdateDialogComponent }}
		<UpdateDialogComponent bind:open={updateDialogOpen} />
	{/await}
{/if}

<style>
	/* file-explorer style indent guides: a vertical rail under the group
	   chevron plus a short stub connecting each row to it; the guides live
	   in the left gutter so the row hover background never covers them,
	   and the last row's rail stops at mid-height ending with an elbow */
	.tree-item {
		position: relative;
	}
	.tree-item::before,
	.tree-item::after {
		content: '';
		position: absolute;
		left: -8px;
	}
	.tree-item::before {
		top: 0;
		bottom: -1px;
		border-left: 1px solid color-mix(in oklch, var(--sidebar-border) 60%, transparent);
	}
	.tree-item::after {
		top: 50%;
		width: 14px;
		border-top: 1px solid color-mix(in oklch, var(--sidebar-border) 60%, transparent);
	}
	.tree-item:last-of-type::before {
		bottom: 50%;
	}
</style>
