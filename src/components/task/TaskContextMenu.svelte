<script lang="ts">
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import type { Task, TaskStatus } from '$lib/types/task';

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

<ContextMenu.Content>
	<ContextMenu.Item onclick={() => onEdit(task)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
			/></svg
		>
		Edit task
	</ContextMenu.Item>
	<ContextMenu.Item onclick={() => onTogglePin(task)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
			/></svg
		>
		{task.pinned ? 'Unpin' : 'Pin'}
	</ContextMenu.Item>
	<ContextMenu.Separator />
	<ContextMenu.Item
		onclick={() => onChangeStatus(task, task.status === 'todo' ? 'in_progress' : 'todo')}
	>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M5.669 4.76a1.469 1.469 0 0 1 2.04-1.177c1.062.454 3.442 1.533 6.462 3.276 3.021 1.744 5.146 3.267 6.069 3.958.788.591.79 1.763.001 2.356-.914.687-3.013 2.19-6.07 3.956-3.06 1.766-5.412 2.832-6.464 3.28-.906.387-1.92-.2-2.038-1.177-.138-1.142-.396-3.735-.396-7.237 0-3.5.257-6.092.396-7.235"
			/></svg
		>
		{task.status === 'todo' ? 'Mark as in progress' : 'Move to todo'}
	</ContextMenu.Item>
	<ContextMenu.Item onclick={() => onChangeStatus(task, 'done')}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m3.535 6.381-4.95 4.95-2.12-2.121a1 1 0 0 0-1.415 1.414l2.758 2.758a1.1 1.1 0 0 0 1.556 0l5.586-5.586a1 1 0 0 0-1.415-1.415"
			/></svg
		>
		Mark as done
	</ContextMenu.Item>
	<ContextMenu.Separator />
	<ContextMenu.Item onclick={() => onDuplicate(task.id)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M9 2a2 2 0 0 0-2 2v2h2V4h11v11h-2v2h2a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM4 7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2z"
			/></svg
		>
		Duplicate
	</ContextMenu.Item>
	<ContextMenu.Item variant="destructive" onclick={() => onDelete(task.id)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
			/></svg
		>
		Delete
	</ContextMenu.Item>
</ContextMenu.Content>
