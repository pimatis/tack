<script lang="ts">
	import { onMount } from 'svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import type { Task, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import { findAll } from '$lib/repositories/project.repository';
	import { bulkUpdateProject, startTimer, stopTimer } from '$lib/repositories/task.repository';
	import { issueId } from '$lib/task/utils';
	import { getSettings } from '$lib/stores/settings';
	import Icon from '$lib/components/Icon.svelte';

	let {
		task,
		onEdit,
		onTogglePin,
		onChangeStatus,
		onDuplicate,
		onDelete
	}: {
		task: Task;
		onEdit: (task: Task) => void;
		onTogglePin: (task: Task) => void;
		onChangeStatus: (task: Task, status: TaskStatus) => void;
		onDuplicate: (id: string) => void;
		onDelete: (id: string) => void;
	} = $props();

	// content mounts on open, so the project list loads lazily with the menu
	let projects = $state<Project[]>([]);
	onMount(() => {
		void findAll().then((list) => (projects = list));
	});

	function moveToProject(projectId: string | null) {
		void bulkUpdateProject([task.id], projectId);
	}

	function toggleTimer() {
		if (task.timerStartedAt) void stopTimer(task.id);
		else void startTimer(task.id);
	}

	function copyAsMarkdown() {
		const checkbox = task.status === 'done' ? '[x]' : task.status === 'canceled' ? '[-]' : '[ ]';
		const line = `- ${checkbox} **${issueId(task, projects, getSettings())}** ${task.title}`;
		void navigator.clipboard.writeText(line);
	}
</script>

<DropdownMenu.Content class="w-52">
	<DropdownMenu.Item onclick={() => onEdit(task)}>
		<Icon icon="pencil" size={16} />
		Edit task
	</DropdownMenu.Item>
	<DropdownMenu.Item onclick={() => onTogglePin(task)}>
		<Icon icon="pin" size={16} />
		{task.pinned ? 'Unpin' : 'Pin'}
	</DropdownMenu.Item>
	<DropdownMenu.Sub>
		<DropdownMenu.SubTrigger>
			<Icon icon="hashtag" size={16} />
			Move to project
		</DropdownMenu.SubTrigger>
		<DropdownMenu.SubContent class="w-44">
			{#each projects as project (project.id)}
				<DropdownMenu.Item onclick={() => moveToProject(project.id)}>
					<Icon icon="hashtag" size={15} />
					{project.name}
					{#if task.projectId === project.id}<Icon icon="check" class="ml-auto" />{/if}
				</DropdownMenu.Item>
			{/each}
			{#if task.projectId}
				<DropdownMenu.Separator />
				<DropdownMenu.Item onclick={() => moveToProject(null)}>
					Remove from project
				</DropdownMenu.Item>
			{/if}
		</DropdownMenu.SubContent>
	</DropdownMenu.Sub>
	<DropdownMenu.Separator />
	<DropdownMenu.Item
		onclick={() => onChangeStatus(task, task.status === 'todo' ? 'in_progress' : 'todo')}
	>
		<Icon icon="play" size={16} />
		{task.status === 'todo' ? 'Mark as in progress' : 'Move to todo'}
	</DropdownMenu.Item>
	<DropdownMenu.Item onclick={() => onChangeStatus(task, 'done')}>
		<Icon icon="check-circle" size={16} />
		Mark as done
	</DropdownMenu.Item>
	<DropdownMenu.Separator />
	<DropdownMenu.Item onclick={toggleTimer}>
		<Icon icon={task.timerStartedAt ? 'stop' : 'play'} size={16} />
		{task.timerStartedAt ? 'Stop timer' : 'Start timer'}
	</DropdownMenu.Item>
	<DropdownMenu.Item onclick={copyAsMarkdown}>
		<Icon icon="copy" size={16} />
		Copy as Markdown
	</DropdownMenu.Item>
	<DropdownMenu.Separator />
	<DropdownMenu.Item onclick={() => onDuplicate(task.id)}>
		<Icon icon="copy" size={16} />
		Duplicate
	</DropdownMenu.Item>
	<DropdownMenu.Item variant="destructive" onclick={() => onDelete(task.id)}>
		<Icon icon="trash" size={16} />
		Delete
	</DropdownMenu.Item>
</DropdownMenu.Content>
