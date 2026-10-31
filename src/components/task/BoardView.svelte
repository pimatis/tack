<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import TaskCard from './TaskCard.svelte';
	import { dropZone, type DragDropState } from '$lib/dnd';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { statusConfig } from '$lib/task/constants';
	import Icon from '$lib/components/Icon.svelte';

	let {
		groups,
		statusOrderList,
		projects,
		appSettings,
		labelMap,
		selectedIds,
		dialogOpen = $bindable(), // eslint-disable-line no-useless-assignment
		onEdit,
		onChangeStatus,
		onChangePriority,
		onTogglePin,
		onDuplicate,
		onDelete,
		onBoardDrop
	}: {
		groups: Record<TaskStatus, Task[]>;
		statusOrderList: TaskStatus[];
		projects: Project[];
		appSettings: Settings;
		labelMap: Map<string, Label>;
		selectedIds: Set<string>;
		dialogOpen: boolean;
		onEdit: (task: Task) => void;
		onChangeStatus: (task: Task, status: TaskStatus) => void;
		onChangePriority: (task: Task, priority: TaskPriority) => void;
		onTogglePin: (task: Task) => void;
		onDuplicate: (id: string) => void;
		onDelete: (id: string) => void;
		onBoardDrop: (state: DragDropState<Task>, targetTask: Task | null) => void;
	} = $props();
</script>

<div class="flex flex-1 gap-3 overflow-x-auto pb-2">
	{#each statusOrderList as status (status)}
		{@const group = groups[status]}
		<div class="flex w-[240px] shrink-0 flex-col sm:w-[260px]">
			<!-- column header -->
			<div class="flex items-center gap-2 px-1 pb-2">
				<StatusIcon {status} size={14} />
				<span class="text-[13px] font-medium text-foreground">{statusConfig[status].label}</span>
				<span class="text-[12px] text-muted-foreground/60">{group.length}</span>
				<div class="flex-1"></div>
				<Button
					variant="ghost"
					size="icon-xs"
					class="flex size-5 items-center justify-center rounded text-muted-foreground/40 transition-colors hover:bg-muted hover:text-foreground"
					aria-label={`Add task to ${statusConfig[status].label}`}
					onclick={() => (dialogOpen = true)}
				>
					<Icon icon="add" size={14} />
				</Button>
			</div>

			<!-- cards -->
			<div
				class="flex flex-1 flex-col gap-1.5 overflow-y-auto"
				use:dropZone={{
					container: `board-${status}`,
					direction: 'vertical',
					onDrop: (state: DragDropState<Task>) => onBoardDrop(state, null)
				}}
			>
				{#if group.length === 0}
					<div
						class="flex flex-col items-center gap-1 rounded-lg border border-dashed border-border/50 py-8 text-center"
					>
						<span class="text-[12px] text-muted-foreground/40">No tasks</span>
					</div>
				{:else}
					{#each group as task (task.id)}
						<TaskCard
							{task}
							{projects}
							{appSettings}
							{labelMap}
							{selectedIds}
							{onEdit}
							{onChangeStatus}
							{onChangePriority}
							{onTogglePin}
							{onDuplicate}
							{onDelete}
							{onBoardDrop}
						/>
					{/each}
				{/if}
			</div>
		</div>
	{/each}
</div>
