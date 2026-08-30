<script lang="ts">
	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import StatusIcon from '../../components/StatusIcon.svelte';
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
	import { searchTaskIds } from '$lib/search/fts.service';
	import { issueId } from '$lib/task/utils';

	let tasks = $state<Task[]>([]);
	let projects = $state<Project[]>([]);
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
		return tasks.filter(
			(t) => `${t.title} ${t.description ?? ''}`.toLowerCase().includes(q)
		);
	});

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
		return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(d);
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

	onMount(() => {
		void load();
		let refreshTimer: ReturnType<typeof setTimeout> | null = null;
		const unlisten = listen('db-changed', () => {
			if (refreshTimer) clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => void load(), 200);
		});
		return () => unlisten.then((fn) => fn());
	});
</script>

<section class="flex h-full flex-col">
	<!-- header -->
	<header class="flex items-center justify-between border-b border-border px-6 py-4">
		<div>
			<div class="flex items-center gap-3">
				<h1 class="text-lg font-semibold tracking-tight">Trash</h1>
				{#if !loading && tasks.length > 0}
					<div class="flex items-center gap-1.5 text-[12px] text-muted-foreground">
						<span class="size-1.5 rounded-full bg-foreground/30"></span>
						<span>{tasks.length} {tasks.length === 1 ? 'item' : 'items'}</span>
					</div>
				{/if}
			</div>
			<p class="text-sm text-muted-foreground">
				Deleted tasks can be restored or permanently removed
			</p>
		</div>
		<div class="flex items-center gap-2">
			{#if tasks.length > 0}
				<Dialog.Root bind:open={emptyConfirmOpen}>
					<Dialog.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="outline"
								size="sm"
								class="text-destructive hover:bg-destructive/5"
							>
								<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
									/></svg
								>
								Empty trash
							</Button>
						{/snippet}
					</Dialog.Trigger>
					<Dialog.Content>
						<Dialog.Header>
							<Dialog.Title>Empty trash</Dialog.Title>
							<Dialog.Description>
								This will permanently delete {tasks.length}
								{tasks.length === 1 ? 'task' : 'tasks'} from the trash. This cannot be undone.
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
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
					/></svg
				>
				Back
			</Button>
		</div>
	</header>

	<!-- content -->
	<div class="flex-1 overflow-y-auto px-6 py-6">
		<div class="mx-auto max-w-3xl">
			{#if loading}
				<div class="flex items-center justify-center gap-2 py-20 text-[13px] text-muted-foreground">
					<Spinner class="size-3.5" />
					<span>Loading trash...</span>
				</div>
			{:else if error}
				<div class="flex flex-col items-center gap-3 py-20">
					<div class="flex size-8 items-center justify-center rounded-lg bg-destructive/10">
						<svg class="text-destructive" width="16" height="16" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m-.01 8H11a1 1 0 0 0-.117 1.993L11 12v4.99c0 .52.394.95.9 1.004l.11.006h.49a1 1 0 0 0 .596-1.803L13 16.134V11.01c0-.52-.394-.95-.9-1.004zM12 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2"
							/></svg
						>
					</div>
					<p class="text-[13px] text-destructive" role="alert">{error}</p>
					<Button variant="outline" size="sm" onclick={load}>Try again</Button>
				</div>
			{:else if tasks.length === 0}
				<div class="flex flex-col items-center justify-center gap-5 py-28">
					<div class="flex size-14 items-center justify-center rounded-2xl bg-muted/50">
						<svg
							class="text-muted-foreground/60"
							width="28"
							height="28"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
							/></svg
						>
					</div>
					<div class="flex flex-col items-center gap-1.5">
						<p class="text-[15px] font-semibold text-foreground">Trash is empty</p>
						<p class="text-[13px] text-muted-foreground">Deleted tasks will appear here</p>
					</div>
				</div>
			{:else}
				<!-- search -->
				<div class="mb-4 flex items-center gap-2">
					<div class="relative flex-1">
						<svg
							class="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground/50"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
							/></svg
						>
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
						<ContextMenu.Root>
							<ContextMenu.Trigger class="contents">
								<article
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
									<div class="flex w-16 shrink-0 justify-end">
										{#if task.projectId}
											{@const project = projectMap.get(task.projectId)}
											{#if project}
												<Badge
													variant="outline"
													class="text-[10px] font-medium text-muted-foreground"
												>
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
							</ContextMenu.Trigger>
							<ContextMenu.Content>
								<ContextMenu.Item onclick={() => void handleRestore(task.id)}>
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
										><path
											fill="currentColor"
											d="M2.614 5.426A1.5 1.5 0 0 1 4 4.5h10a7.5 7.5 0 1 1 0 15H5a1.5 1.5 0 0 1 0-3h9a4.5 4.5 0 1 0 0-9H7.621l.94.94a1.5 1.5 0 0 1-2.122 2.12l-3.5-3.5a1.5 1.5 0 0 1-.325-1.634Z"
										/></svg
									>
									Restore
								</ContextMenu.Item>
								<ContextMenu.Separator />
								<ContextMenu.Item
									variant="destructive"
									onclick={() => void handlePermanentDelete(task.id)}
								>
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
										><path
											fill="currentColor"
											d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
										/></svg
									>
									Delete forever
								</ContextMenu.Item>
							</ContextMenu.Content>
						</ContextMenu.Root>
					{/each}
				</div>

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
