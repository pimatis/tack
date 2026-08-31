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
	import StatusIcon from './StatusIcon.svelte';

	let open = $state(false);
	let tasks = $state<Task[]>([]);
	let projects = $state<Project[]>([]);

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

		const unregisterCommandPalette = registry.register({
			id: 'command-palette',
			run: () => (open = !open)
		});

		const unregisterNewTask = registry.register({
			id: 'new-task',
			enabled: () => open || !document.querySelector("[role='dialog'] input"),
			run: () => newTask()
		});

		const unregisterNewProject = registry.register({
			id: 'new-project',
			enabled: () => open || !document.querySelector("[role='dialog'] input"),
			run: () => newProject()
		});

		const handleOpenPalette = () => (open = true);
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
	title="Command palette"
	description="Search for commands, tasks, and projects"
	showCloseButton={false}
	class="top-[18%]! max-w-[560px]!"
>
	<Command.Input placeholder="Type a command or search..." />
	<Command.List class="max-h-[400px]">
		<Command.Empty>No results found.</Command.Empty>

		<Command.Group heading="Actions">
			<Command.Item onSelect={() => newTask()}>
				<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
					/></svg
				>
				<span>New task</span>
				<Command.Shortcut>C</Command.Shortcut>
			</Command.Item>
			<Command.Item onSelect={() => newProject()}>
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

		{#if tasks.length > 0}
			<Command.Separator />
			<Command.Group heading="Tasks">
				{#each tasks as task (task.id)}
					<Command.Item
						value={`${issueId(task, projects, getSettings())} ${task.title}`}
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

		{#if projects.length > 0}
			<Command.Separator />
			<Command.Group heading="Projects">
				{#each projects as project (project.id)}
					<Command.Item
						value={`${project.name} ${project.prefix}`}
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
	</Command.List>
</Command.Dialog>
