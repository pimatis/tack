<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from '../../components/task/StatusIcon.svelte';
	import {
		findTrashed,
		restore as restoreTask,
		permanentDelete,
		emptyTrash
	} from '$lib/repositories/task.repository';
	import { findAll as findProjects } from '$lib/repositories/project.repository';
	import type { Task } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import { getSettings } from '$lib/stores/settings';
	import { onDbChanged } from '$lib/db/client';
	import { searchTaskIds } from '$lib/search/fts.service';
	import { issueId } from '$lib/task/utils';
	import Icon from '$lib/components/Icon.svelte';

	let tasks = $state<Task[]>([]);
	let projects = $state<Project[]>([]);
	// start on the filter matching the tab the trash was opened from
	let loading = $state(true);
	let error = $state<string | null>(null);
	let searchQuery = $state('');
	let ftsIds = $state<Set<string> | null>(null);
	let emptyConfirmOpen = $state(false);
	let appSettings = $state(getSettings());

	let filteredTasks = $derived.by(() => {
		const q = searchQuery.toLowerCase().trim();
		if (!q) return tasks;
		// fts match (title, description, subtasks) with in-memory fallback while pending
		if (ftsIds) return tasks.filter((t) => ftsIds?.has(t.id));
		return tasks.filter((t) => `${t.title} ${t.description ?? ''}`.toLowerCase().includes(q));
	});

	let totalCount = $derived(tasks.length);

	$effect(() => {
		const query = searchQuery;
		const timer = setTimeout(() => {
			if (!query.trim()) {
				ftsIds = null;
				return;
			}
			searchTaskIds(query)
				.then((ids) => (ftsIds = ids))
				.catch(() => (ftsIds = null));
		}, 150);
		return () => clearTimeout(timer);
	});

	let projectMap = $derived(new Map(projects.map((p) => [p.id, p])));

	function formatDate(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diffMs = now.getTime() - d.getTime();
		const diffMin = Math.floor(diffMs / 60000);
		const diffHr = Math.floor(diffMin / 60);
		const diffDay = Math.floor(diffHr / 24);
		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		if (diffHr < 24) return `${diffHr}h ago`;
		if (diffDay < 7) return `${diffDay}d ago`;
		return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric' }).format(d);
	}

	async function load() {
		loading = true;
		error = null;
		try {
			const [t, p] = await Promise.all([findTrashed(), findProjects()]);
			tasks = t;
			projects = p;
		} catch (e) {
			error = 'Failed to load trash';
			console.error(e);
		}
		loading = false;
	}

	async function handleRestore(id: string) {
		try {
			await restoreTask(id);
			await load();
			window.dispatchEvent(new Event('tasks-changed'));
		} catch (e) {
			error = 'Failed to restore task';
			console.error(e);
		}
	}

	async function handlePermanentDelete(id: string) {
		try {
			await permanentDelete(id);
			await load();
			window.dispatchEvent(new Event('tasks-changed'));
		} catch (e) {
			error = 'Failed to delete task';
			console.error(e);
		}
	}

	async function handleEmptyTrash() {
		try {
			await emptyTrash();
			emptyConfirmOpen = false;
			await load();
			window.dispatchEvent(new Event('tasks-changed'));
		} catch (e) {
			error = 'Failed to empty trash';
			console.error(e);
		}
	}

	// right-click on a row opens the dropdown at the pointer
	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);
	let menuTask = $state<Task | null>(null);

	function openTaskMenu(event: MouseEvent, task: Task) {
		event.preventDefault();
		menuTask = task;
		menuX = event.clientX;
		menuY = event.clientY;
		menuOpen = true;
	}

	onMount(() => {
		void load();
		let refreshTimer: ReturnType<typeof setTimeout> | null = null;
		const unlisten = onDbChanged(() => {
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => void load(), 200);
		});
		return () => unlisten();
	});
</script>

<section class="flex h-full flex-col">
	<!-- header -->
	<header
		class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-6 sm:py-4"
	>
		<div class="min-w-0">
			<div class="flex items-center gap-3">
				<h1 class="text-base font-semibold tracking-tight sm:text-lg">Trash</h1>
				{#if !loading && totalCount > 0}
					<div class="flex items-center gap-1.5 text-[12px] text-muted-foreground">
						<span class="size-1.5 rounded-full bg-foreground/30"></span>
						<span
							>{filteredTasks.length}
							{filteredTasks.length === 1 ? 'item' : 'items'}</span
						>
					</div>
				{/if}
			</div>
			<p class="truncate text-xs text-muted-foreground sm:text-sm">
				Deleted tasks can be restored or permanently removed
			</p>
		</div>
		<div class="flex shrink-0 items-center gap-2">
			{#if totalCount > 0}
				<Dialog.Root bind:open={emptyConfirmOpen}>
					<Dialog.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="outline"
								size="sm"
								class="text-destructive hover:bg-destructive/5"
							>
								<Icon icon="trash" size={14} />
								Empty trash
							</Button>
						{/snippet}
					</Dialog.Trigger>
					<Dialog.Content>
						<Dialog.Header>
							<Dialog.Title>Empty trash</Dialog.Title>
							<Dialog.Description>
								This will permanently delete {totalCount}
								{totalCount === 1 ? 'item' : 'items'} from the trash. This cannot be undone.
							</Dialog.Description>
						</Dialog.Header>
						<Dialog.Footer>
							<Dialog.Close>
								{#snippet child({ props })}
									<Button {...props} variant="outline" size="sm">Cancel</Button>
								{/snippet}
							</Dialog.Close>
							<Button variant="destructive" size="sm" onclick={() => void handleEmptyTrash()}>
								Delete forever
							</Button>
						</Dialog.Footer>
					</Dialog.Content>
				</Dialog.Root>
			{/if}
			<Button variant="ghost" size="sm" href="/">
				<Icon icon="arrow-right" size={16} />
				Back
			</Button>
		</div>
	</header>

	<!-- content -->
	<div class="flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
		<div class="mx-auto max-w-3xl">
			{#if loading}
				<div class="flex items-center justify-center gap-2 py-20 text-[13px] text-muted-foreground">
					<Spinner class="size-3.5" />
					<span>Loading trash...</span>
				</div>
			{:else if error}
				<div class="flex flex-col items-center gap-3 py-20">
					<div class="flex size-8 items-center justify-center rounded-lg bg-destructive/10">
						<Icon icon="close-circle" size={16} class="text-destructive" />
					</div>
					<p class="text-[13px] text-destructive" role="alert">{error}</p>
					<Button variant="outline" size="sm" onclick={load}>Try again</Button>
				</div>
			{:else if totalCount === 0}
				<div class="flex flex-col items-center justify-center gap-5 py-28">
					<div class="flex size-14 items-center justify-center rounded-2xl bg-muted/50">
						<Icon icon="trash" size={28} class="text-muted-foreground/60" />
					</div>
					<div class="flex flex-col items-center gap-1.5">
						<p class="text-[15px] font-semibold text-foreground">Trash is empty</p>
						<p class="text-[13px] text-muted-foreground">Deleted tasks will appear here</p>
					</div>
				</div>
			{:else}
				<!-- search + type filter -->
				<div class="mb-4 flex items-center gap-2">
					<div class="relative flex-1">
						<Icon
							icon="search"
							class="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground/50"
						/>
						<Input
							placeholder="Search in trash..."
							bind:value={searchQuery}
							class="h-8 w-full rounded-lg border border-input bg-transparent pr-3 pl-8 text-[13px] text-foreground transition-all outline-none placeholder:text-muted-foreground/50 dark:bg-input/30"
						/>
					</div>
				</div>

				<Separator class="mb-4" />

				<!-- task list -->
				<div class="flex flex-col">
					{#each filteredTasks as task (task.id)}
						<article
							oncontextmenu={(e) => openTaskMenu(e, task)}
							class="group/task -mx-2 flex items-center gap-2.5 rounded-lg px-2 py-2 transition-colors hover:bg-muted/40"
						>
							<!-- status icon -->
							<span class="flex size-5 shrink-0 items-center justify-center">
								<StatusIcon status={task.status} size={14} />
							</span>

							<!-- issue id -->
							<span class="shrink-0 font-mono text-[11px] font-medium text-muted-foreground/50"
								>{issueId(task, projects, appSettings)}</span
							>

							<!-- title -->
							<span
								class="min-w-0 flex-1 truncate text-[13px] text-foreground/70 {task.status ===
								'canceled'
									? 'text-muted-foreground/40 line-through'
									: ''}"
							>
								{task.title}
							</span>

							<!-- project badge -->
							<div class="hidden w-16 shrink-0 justify-end md:flex">
								{#if task.projectId}
									{@const project = projectMap.get(task.projectId)}
									{#if project}
										<Badge variant="outline" class="text-[10px] font-medium text-muted-foreground">
											{project.prefix}
										</Badge>
									{/if}
								{/if}
							</div>

							<!-- deleted date -->
							<span
								class="hidden w-16 shrink-0 text-right text-[11px] text-muted-foreground/40 sm:block"
							>
								{task.deletedAt ? formatDate(task.deletedAt) : ''}
							</span>
						</article>
					{/each}
				</div>

				<DropdownMenu.Root bind:open={menuOpen}>
					<DropdownMenu.Trigger
						class="h-0 w-0 outline-none"
						style="position: fixed; left: {menuX}px; top: {menuY}px"
					/>
					<DropdownMenu.Content class="w-44">
						{#if menuTask}
							<DropdownMenu.Item onclick={() => void handleRestore(menuTask!.id)}>
								<Icon icon="undo" size={16} />
								Restore
							</DropdownMenu.Item>
							<DropdownMenu.Separator />
							<DropdownMenu.Item
								variant="destructive"
								onclick={() => void handlePermanentDelete(menuTask!.id)}
							>
								<Icon icon="trash" size={16} />
								Delete forever
							</DropdownMenu.Item>
						{/if}
					</DropdownMenu.Content>
				</DropdownMenu.Root>

				{#if filteredTasks.length === 0}
					<div class="flex flex-col items-center gap-3 py-16">
						<p class="text-[13px] text-muted-foreground">No matching items.</p>
						<Button
							variant="ghost"
							size="sm"
							class="h-auto p-0 text-[12px] font-medium text-foreground/70 transition-colors hover:text-foreground"
							onclick={() => (searchQuery = '')}
						>
							Clear search
						</Button>
					</div>
				{/if}
			{/if}
		</div>
	</div>
</section>
