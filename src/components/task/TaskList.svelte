<script lang="ts">
	import StatusIcon from '../StatusIcon.svelte';
	import TaskRow from './TaskRow.svelte';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import type { DragDropState } from '$lib/dnd';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { statusConfig } from '$lib/task/constants';

	let {
		groups,
		statusOrderList,
		collapsedStatuses,
		onToggleCollapse,
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
		groups: Record<TaskStatus, Task[]>;
		statusOrderList: TaskStatus[];
		collapsedStatuses: Set<TaskStatus>;
		onToggleCollapse: (status: TaskStatus) => void;
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

<div class="flex flex-col gap-5">
	{#each statusOrderList as status (status)}
		{@const group = groups[status]}
		{@const collapsed = collapsedStatuses.has(status)}
		{#if group.length > 0}
			<div>
				<!-- group header -->
				<button
					onclick={() => onToggleCollapse(status)}
					class="flex w-full items-center gap-2 rounded-md px-1 pt-0.5 pb-1.5 text-left transition-colors hover:bg-muted/40"
				>
					<StatusIcon {status} size={14} />
					<span class="text-[13px] font-medium text-foreground">{statusConfig[status].label}</span>
					<span class="text-[12px] text-muted-foreground/60">{group.length}</span>
					<ChevronDownIcon
						class="ml-auto size-3.5 text-muted-foreground/50 transition-transform {collapsed
							? '-rotate-90'
							: ''}"
					/>
				</button>

				<!-- task rows -->
				{#if !collapsed}
					<div class="flex flex-col">
						{#each group as task (task.id)}
							<TaskRow
								{task}
								{projects}
								{appSettings}
								{labelMap}
								{selectedIds}
								{onToggleSelect}
								{onChangePriority}
								{onChangeStatus}
								{onEdit}
								{onTogglePin}
								{onDuplicate}
								{onDelete}
								{onListDrop}
							/>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	{/each}
</div>
