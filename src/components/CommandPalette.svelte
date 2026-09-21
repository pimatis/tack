<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { findAll as findAllTasks } from '$lib/repositories/task.repository';
	import { findAll as findAllProjects } from '$lib/repositories/project.repository';
	import type { Task } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import * as Command from '$lib/components/ui/command/index.js';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import { issueId } from '$lib/task/utils';
	import { getSettings } from '$lib/stores/settings';
	import { notesState } from '$lib/notes/notesState.svelte';
	import { searchNotes, type NoteSearchResult } from '$lib/notes/search';
	import { searchTaskIds } from '$lib/search/fts.service';
	import StatusIcon from './StatusIcon.svelte';

	let open = $state(false);
	let query = $state('');
	let tasks = $state<Task[]>([]);
	let projects = $state<Project[]>([]);
	let noteResults = $state<NoteSearchResult[] | null>(null);
	let taskResults = $state<Set<string> | null>(null);

	const normalized = $derived(query.trim().toLowerCase());

	// cmdk's built-in filter cannot mix with async fts results, so filter here.
	// fts hits add description, label, project and issue number matches; the
	// in-memory title/issue substring keeps partial words working while the
	// async results are still in flight
	const shownTasks = $derived.by(() => {
		if (!normalized) return tasks;
		const ids = taskResults;
		return tasks.filter(
			(t) =>
				(ids?.has(t.id) ?? false) ||
				`${issueId(t, projects, getSettings())} ${t.title}`.toLowerCase().includes(normalized)
		);
	});

	const shownProjects = $derived(
		normalized
			? projects.filter((p) => `${p.name} ${p.prefix}`.toLowerCase().includes(normalized))
			: projects
	);

	// notes: fts hits while typing, the plain root list when the query is empty
	const shownNotes = $derived<NoteSearchResult[]>(
		normalized
			? (noteResults ?? [])
			: notesState.folder
				? notesState.notes.map((n) => ({ path: n.path, name: n.name, snippet: '' }))
				: []
	);

	async function goHomeThenDispatch(eventName: string, detail?: unknown) {
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

	async function loadData() {
		try {
			[tasks, projects] = await Promise.all([findAllTasks(), findAllProjects()]);
		} catch {
			// ignore
		}
		if (notesState.folder) void notesState.refresh();
	}

	$effect(() => {
		if (open) void loadData();
	});

	// debounced full-text search over note names and content
	$effect(() => {
		const q = normalized;
		if (!open || !q) {
			noteResults = null;
			return;
		}
		const timer = setTimeout(async () => {
			noteResults = await searchNotes(q).catch(() => []);
		}, 150);
		return () => clearTimeout(timer);
	});

	// debounced full-text search over tasks
	$effect(() => {
		const q = normalized;
		if (!open || !q) {
			taskResults = null;
			return;
		}
		const timer = setTimeout(async () => {
			taskResults = await searchTaskIds(q).catch(() => null);
		}, 150);
		return () => clearTimeout(timer);
	});

	// folder path of a note relative to the notes root ('' for root notes)
	function folderLabel(path: string): string {
		const root = notesState.folder?.replace(/\/+$/, '');
		if (!root || !path.startsWith(`${root}/`)) return '';
		const parts = path.slice(root.length + 1).split('/');
		parts.pop();
		return parts.join(' / ');
	}

	function selectTask(task: Task) {
		void goHomeThenDispatch('edit-task-from-command', task);
		open = false;
	}

	function selectProject(project: Project) {
		void goHomeThenDispatch('filter-by-project', project.id);
		open = false;
	}

	async function selectNote(path: string) {
		notesState.activeTab = 'notes';
		if (window.location.pathname !== '/') {
			await goto('/');
			await tick();
		}
		void notesState.openNote(path);
		open = false;
	}

	function newTask() {
		void goHomeThenDispatch('open-task-dialog');
		open = false;
	}

	function newProject() {
		void goHomeThenDispatch('open-project-dialog');
		open = false;
	}

	onMount(() => {
		const registry = getShortcutRegistry();

		// one unified search over tasks, projects and notes
		const openPalette = () => (open = !open);

		const unregisterCommandPalette = registry.register({
			id: 'command-palette',
			run: openPalette
		});

		// Cmd+P stays a notes-tab shortcut and opens the same unified search
		const unregisterNotesSearch = registry.register({
			id: 'notes-search',
			enabled: () => notesState.activeTab === 'notes',
			run: () => (open = true)
		});

		const unregisterNewTask = registry.register({
			id: 'new-task',
			enabled: () =>
				window.location.pathname === '/' &&
				(open || !document.querySelector("[role='dialog'] input")),
			run: () => newTask()
		});

		const unregisterNewProject = registry.register({
			id: 'new-project',
			enabled: () =>
				window.location.pathname === '/' &&
				(open || !document.querySelector("[role='dialog'] input")),
			run: () => newProject()
		});

		const handleOpenPalette = openPalette;
		window.addEventListener('open-command-palette', handleOpenPalette);
		return () => {
			unregisterCommandPalette();
			unregisterNotesSearch();
			unregisterNewTask();
			unregisterNewProject();
			window.removeEventListener('open-command-palette', handleOpenPalette);
		};
	});
</script>

<Command.Dialog
	bind:open
	shouldFilter={false}
	title="Search"
	description="Search tasks, projects, and notes"
	showCloseButton={false}
	class="top-[12%]! w-[calc(100vw-2rem)]! max-w-[560px]! sm:top-[18%]"
>
	<Command.Input bind:value={query} placeholder="Search tasks, projects, and notes..." />
	<Command.List class="max-h-[60vh] sm:max-h-[400px]">
		<Command.Group heading="Actions">
			<Command.Item value="new-task" onSelect={() => newTask()}>
				<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
					/></svg
				>
				<span>New task</span>
				<Command.Shortcut>C</Command.Shortcut>
			</Command.Item>
			<Command.Item value="new-project" onSelect={() => newProject()}>
				<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
					/></svg
				>
				<span>New project</span>
				<Command.Shortcut>N</Command.Shortcut>
			</Command.Item>
		</Command.Group>

		{#if shownTasks.length > 0}
			<Command.Separator />
			<Command.Group heading="Tasks">
				{#each shownTasks as task (task.id)}
					<Command.Item
						value={task.id}
						onSelect={() => selectTask(task)}
						class="[&_.cn-command-item-indicator]:hidden"
					>
						<StatusIcon status={task.status} size={12} />
						<span class="min-w-0 flex-1 truncate">{task.title}</span>
						<span class="ml-auto shrink-0 font-mono text-[11px] text-muted-foreground/50"
							>{issueId(task, projects, getSettings())}</span
						>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}

		{#if shownProjects.length > 0}
			<Command.Separator />
			<Command.Group heading="Projects">
				{#each shownProjects as project (project.id)}
					<Command.Item
						value={project.id}
						onSelect={() => selectProject(project)}
						class="[&_.cn-command-item-indicator]:hidden"
					>
						<svg
							class="text-muted-foreground"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M3 4.5A1.5 1.5 0 0 1 4.5 3h15A1.5 1.5 0 0 1 21 4.5v2.086A2 2 0 0 1 20.414 8L15 13.414v7.424a1.1 1.1 0 0 1-1.592.984l-3.717-1.858A1.25 1.25 0 0 1 9 18.846v-5.432L3.586 8A2 2 0 0 1 3 6.586z"
							/></svg
						>
						<span class="min-w-0 flex-1 truncate">{project.name}</span>
						<span class="ml-auto shrink-0 text-[11px] text-muted-foreground/50"
							>{project.prefix}</span
						>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}

		{#if shownNotes.length > 0}
			<Command.Separator />
			<Command.Group heading="Notes">
				{#each shownNotes as note (note.path)}
					<Command.Item
						value={note.path}
						onSelect={() => selectNote(note.path)}
						class="[&_.cn-command-item-indicator]:hidden"
					>
						<svg
							class="mt-0.5 text-muted-foreground"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5zM8 12h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2m0 4h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2"
							/></svg
						>
						<span class="flex min-w-0 flex-col">
							<span class="truncate">{note.name.replace(/\.md$/, '')}</span>
							{#if folderLabel(note.path)}
								<span class="truncate text-[10px] text-muted-foreground/60">
									{folderLabel(note.path)}
								</span>
							{/if}
							{#if note.snippet}
								<span class="truncate text-[11px] text-muted-foreground">{note.snippet}</span>
							{/if}
						</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
