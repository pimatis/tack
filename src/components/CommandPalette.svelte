<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { findAll as findAllTasks } from '$lib/repositories/task.repository';
	import { findAll as findAllProjects } from '$lib/repositories/project.repository';
	import type { Task } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import * as Command from '$lib/components/ui/command/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import { issueId } from '$lib/task/utils';
	import { getSettings } from '$lib/stores/settings';
	import { searchTaskIds } from '$lib/search/fts.service';
	import StatusIcon from './task/StatusIcon.svelte';
	import Icon from '$lib/components/Icon.svelte';

	let open = $state(false);
	let query = $state('');
	let tasks = $state<Task[]>([]);
	let projects = $state<Project[]>([]);
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
	}

	$effect(() => {
		if (open) void loadData();
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

	function selectTask(task: Task) {
		void goHomeThenDispatch('edit-task-from-command', task);
		open = false;
	}

	function selectProject(project: Project) {
		void goHomeThenDispatch('filter-by-project', project.id);
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

		// one unified search over tasks and projects
		const openPalette = () => (open = !open);

		const unregisterCommandPalette = registry.register({
			id: 'command-palette',
			run: openPalette
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
	description="Search tasks and projects"
	showCloseButton={false}
	class="top-[12%]! w-[calc(100vw-2rem)]! max-w-[560px]! sm:top-[18%]"
>
	<Command.Input bind:value={query} placeholder="Search tasks and projects..." />
	<Command.List class="max-h-[60vh] sm:max-h-[400px]">
		<Command.Group heading="Actions">
			<Command.Item value="new-task" onSelect={() => newTask()}>
				<Icon icon="compose" size={16} class="text-muted-foreground" />
				<span>New task</span>
				<Command.Shortcut>C</Command.Shortcut>
			</Command.Item>
			<Command.Item value="new-project" onSelect={() => newProject()}>
				<Icon icon="add" size={16} class="text-muted-foreground" />
				<span>New project</span>
				<Command.Shortcut>N</Command.Shortcut>
			</Command.Item>
		</Command.Group>

		{#if shownTasks.length > 0}
			<Command.Separator />
			<Command.Group value="Tasks">
				<Collapsible.Root open class="group/tasks">
					<Collapsible.Trigger
						class="flex w-full cursor-pointer list-none items-center gap-1 rounded-sm px-2 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
					>
						<Icon
							icon="arrow-right"
							size={14}
							class="text-muted-foreground transition-transform duration-150 group-data-[state=open]/tasks:rotate-90"
						/>
						<span>Tasks</span>
					</Collapsible.Trigger>
					<Collapsible.Content>
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
					</Collapsible.Content>
				</Collapsible.Root>
			</Command.Group>
		{/if}

		{#if shownProjects.length > 0}
			<Command.Separator />
			<Command.Group value="Projects">
				<Collapsible.Root open class="group/projects">
					<Collapsible.Trigger
						class="flex w-full cursor-pointer list-none items-center gap-1 rounded-sm px-2 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
					>
						<Icon
							icon="arrow-right"
							size={14}
							class="text-muted-foreground transition-transform duration-150 group-data-[state=open]/projects:rotate-90"
						/>
						<span>Projects</span>
					</Collapsible.Trigger>
					<Collapsible.Content>
						{#each shownProjects as project (project.id)}
							<Command.Item
								value={project.id}
								onSelect={() => selectProject(project)}
								class="[&_.cn-command-item-indicator]:hidden"
							>
								<Icon icon="hashtag" size={14} class="text-muted-foreground" />
								<span class="min-w-0 flex-1 truncate">{project.name}</span>
								<span class="ml-auto shrink-0 text-[11px] text-muted-foreground/50"
									>{project.prefix}</span
								>
							</Command.Item>
						{/each}
					</Collapsible.Content>
				</Collapsible.Root>
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
