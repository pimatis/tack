<script lang="ts">
	import StatusIcon from '../StatusIcon.svelte';
	import TaskRow from './TaskRow.svelte';
	import type { DragDropState } from '$lib/dnd';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { statusConfig } from '$lib/task/constants';

	let {
		groups,
		statusOrderList,
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
		{#if group.length > 0}
			<div>
				<!-- group header -->
				<div class="flex items-center gap-2 px-1 pb-1.5">
					<StatusIcon {status} size={14} />
					<span class="text-[13px] font-medium text-foreground">{statusConfig[status].label}</span>
					<span class="text-[12px] text-muted-foreground/60">{group.length}</span>
				</div>

				<!-- task rows -->
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
			</div>
		{/if}
	{/each}
</div>
