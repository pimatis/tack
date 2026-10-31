<script lang="ts">
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import type { Task, TaskStatus } from '$lib/types/task';
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
	<DropdownMenu.Item onclick={() => onDuplicate(task.id)}>
		<Icon icon="copy" size={16} />
		Duplicate
	</DropdownMenu.Item>
	<DropdownMenu.Item variant="destructive" onclick={() => onDelete(task.id)}>
		<Icon icon="trash" size={16} />
		Delete
	</DropdownMenu.Item>
</DropdownMenu.Content>
