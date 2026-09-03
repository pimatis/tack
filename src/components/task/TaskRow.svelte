<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import StatusIcon from '../StatusIcon.svelte';
	import PriorityIcon from '../PriorityIcon.svelte';
	import PriorityMenu from '../PriorityMenu.svelte';
	import StatusMenu from '../StatusMenu.svelte';
	import TaskLabels from './TaskLabels.svelte';
	import DueDateBadge from './DueDateBadge.svelte';
	import EndDateBadge from './EndDateBadge.svelte';
	import TaskContextMenu from './TaskContextMenu.svelte';
	import { sortableItem, type DragDropState } from '$lib/dnd';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { issueId, formatDate } from '$lib/task/utils';

	let {
		task,
		projects,
		appSettings,
		labelMap,
		selectedIds,
		onToggleSelect,
		onChangePriority,
		onChangeStatus,
		onEdit,
		onTogglePin,
		onDuplicate,
		onDelete,
		onListDrop
	}: {
		task: Task;
		projects: Project[];
		appSettings: Settings;
		labelMap: Map<string, Label>;
		selectedIds: Set<string>;
		onToggleSelect: (taskId: string, shiftKey: boolean) => void;
		onChangePriority: (task: Task, priority: TaskPriority) => void;
		onChangeStatus: (task: Task, status: TaskStatus) => void;
		onEdit: (task: Task) => void;
		onTogglePin: (task: Task) => void;
		onDuplicate: (id: string) => void;
		onDelete: (id: string) => void;
		onListDrop: (state: DragDropState<Task>, targetTask: Task) => void;
	} = $props();
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="contents">
		<article
			use:sortableItem={{
				dragData: task,
				container: `list-${task.status}`,
				onDrop: (state: DragDropState<Task>) => onListDrop(state, task)
			}}
			class="group/task -mx-2 flex cursor-grab items-center gap-2.5 rounded-lg px-2 py-1.5 transition-colors hover:bg-muted/40 active:cursor-grabbing {selectedIds.has(
				task.id
			)
				? 'bg-primary/8'
				: ''}"
		>
			<!-- selection checkbox -->
			<Checkbox
				checked={selectedIds.has(task.id)}
				class="shrink-0 {selectedIds.has(task.id)
					? 'opacity-100'
					: 'opacity-0 group-hover/task:opacity-100'} transition-opacity"
				onclick={(e) => {
					e.preventDefault();
					onToggleSelect(task.id, e.shiftKey);
				}}
			/>

			<!-- priority popover -->
			<PriorityMenu
				value={task.priority}
				onSelect={(p) => onChangePriority(task, p as TaskPriority)}
			>
				{#snippet trigger(props)}
					<Button
						{...props}
						variant="ghost"
						size="icon-xs"
						class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/50 transition-colors hover:bg-muted hover:text-muted-foreground"
						aria-label={`Set priority for ${task.title}`}
					>
						{#if task.priority > 0}
							<PriorityIcon priority={task.priority} size={14} />
						{/if}
					</Button>
				{/snippet}
			</PriorityMenu>

			<!-- issue id (min-width keeps the status icon column aligned across rows) -->
			<span class="min-w-11 shrink-0 font-mono text-[11px] font-medium text-muted-foreground/50">
				{issueId(task, projects, appSettings)}
			</span>

			<!-- labels -->
			<div class="hidden lg:block">
				<TaskLabels labelIds={task.labelIds ?? []} {labelMap} max={3} />
			</div>

			<!-- status popover -->
			<StatusMenu value={task.status} onSelect={(s) => onChangeStatus(task, s)}>
				{#snippet trigger(props)}
					<Button
						{...props}
						variant="ghost"
						size="icon-xs"
						class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/50 transition-colors hover:bg-muted hover:text-muted-foreground"
						aria-label={`Change status for ${task.title}`}
					>
						<StatusIcon status={task.status} size={14} />
					</Button>
				{/snippet}
			</StatusMenu>

			<!-- title -->
			<Button
				variant="ghost"
				class="h-auto min-w-0 flex-1 justify-start overflow-hidden p-0 text-left text-[13px] text-foreground/90 {task.status ===
				'canceled'
					? 'text-muted-foreground/50'
					: ''}"
				onclick={() => onEdit(task)}
			>
				<span
					class="block min-w-0 flex-1 truncate [mask-image:linear-gradient(to_right,black_95%,transparent_100%)] [-webkit-mask-image:linear-gradient(to_right,black_95%,transparent_100%)] {task.status ===
					'canceled'
						? 'line-through'
						: ''}"
				>
					{task.title}
				</span>
			</Button>

			<!-- due date -->
			<div class="hidden w-24 shrink-0 justify-end md:flex">
				<DueDateBadge dueDate={task.dueDate} />
			</div>

			<!-- end date -->
			<div class="hidden w-24 shrink-0 justify-end md:flex">
				<EndDateBadge endDate={task.endDate} />
			</div>

			<!-- date -->
			<span class="hidden w-16 shrink-0 text-right text-[11px] text-muted-foreground/40 md:block">
				{formatDate(task.updatedAt)}
			</span>

			<!-- pinned indicator -->
			<div class="flex w-5 shrink-0 justify-center">
				{#if task.pinned}
					<svg class="text-foreground/60" width="13" height="13" viewBox="0 0 24 24" fill="none">
						<path
							fill="currentColor"
							d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
						/>
					</svg>
				{/if}
			</div>
		</article>
	</ContextMenu.Trigger>
	<TaskContextMenu {task} {onEdit} {onTogglePin} {onChangeStatus} {onDuplicate} {onDelete} />
</ContextMenu.Root>
