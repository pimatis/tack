<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import PriorityIcon from './PriorityIcon.svelte';
	import TaskCard from './TaskCard.svelte';
	import { dropZone, type DragDropState } from '$lib/dnd';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { statusConfig, priorityConfig } from '$lib/task/constants';
	import { labelColorMap } from '$lib/types/label';
	import { agentColor } from '$lib/agents';
	import type { BoardGroupBy } from '$lib/task/taskState.svelte';
	import Icon from '$lib/components/Icon.svelte';

	let {
		tasks,
		groups,
		statusOrderList,
		projects,
		appSettings,
		labelMap,
		selectedIds,
		groupBy = $bindable('status'),
		dialogOpen = $bindable(), // eslint-disable-line no-useless-assignment
		onEdit,
		onChangeStatus,
		onChangePriority,
		onTogglePin,
		onDuplicate,
		onDelete,
		onBoardDrop,
		onFieldDrop
	}: {
		tasks: Task[];
		groups: Record<TaskStatus, Task[]>;
		statusOrderList: TaskStatus[];
		projects: Project[];
		appSettings: Settings;
		labelMap: Map<string, Label>;
		selectedIds: Set<string>;
		groupBy: BoardGroupBy;
		dialogOpen: boolean;
		onEdit: (task: Task) => void;
		onChangeStatus: (task: Task, status: TaskStatus) => void;
		onChangePriority: (task: Task, priority: TaskPriority) => void;
		onTogglePin: (task: Task) => void;
		onDuplicate: (id: string) => void;
		onDelete: (id: string) => void;
		onBoardDrop: (state: DragDropState<Task>, targetTask: Task | null) => void;
		onFieldDrop: (state: DragDropState<Task>, columnKey: string) => void;
	} = $props();

	type BoardColumn = { key: string; title: string; tasks: Task[] };

	const GROUP_LABELS: Record<BoardGroupBy, string> = {
		status: 'Status',
		assignee: 'Assignee',
		priority: 'Priority',
		label: 'Label'
	};

	const columns = $derived.by((): BoardColumn[] => {
		if (groupBy === 'status') {
			return statusOrderList.map((status) => ({
				key: status,
				title: statusConfig[status].label,
				tasks: groups[status]
			}));
		}
		if (groupBy === 'priority') {
			// urgent first, "no priority" last
			return ([1, 2, 3, 4, 0] as const).map((p) => ({
				key: String(p),
				title: priorityConfig[p].label,
				tasks: tasks.filter((t) => t.priority === p)
			}));
		}
		if (groupBy === 'assignee') {
			const names = [...new Set(tasks.map((t) => t.assignee).filter((a): a is string => !!a))];
			names.sort();
			return [
				{ key: '', title: 'Unassigned', tasks: tasks.filter((t) => !t.assignee) },
				...names.map((name) => ({
					key: name,
					title: name,
					tasks: tasks.filter((t) => t.assignee === name)
				}))
			];
		}
		// label
		const labels = [...labelMap.values()].sort((a, b) => a.name.localeCompare(b.name));
		return [
			{ key: '', title: 'No label', tasks: tasks.filter((t) => !(t.labelIds ?? []).length) },
			...labels.map((label) => ({
				key: label.id,
				title: label.name,
				tasks: tasks.filter((t) => (t.labelIds ?? []).includes(label.id))
			}))
		];
	});

	// the dnd container id of a column; status mode keeps the historical names
	function containerFor(column: BoardColumn): string {
		return `board-${column.key}`;
	}

	function handleDrop(column: BoardColumn, state: DragDropState<Task>, task: Task | null) {
		if (groupBy === 'status') onBoardDrop(state, task);
		else onFieldDrop(state, column.key);
	}
</script>

<div class="flex h-full flex-col">
	<div class="flex items-center justify-end gap-2 pb-2">
		<span class="text-[12px] text-muted-foreground">Group by</span>
		<Select.Root type="single" value={groupBy} onValueChange={(v) => (groupBy = v as BoardGroupBy)}>
			<Select.Trigger class="h-7 w-32 text-[12px]">
				{GROUP_LABELS[groupBy]}
			</Select.Trigger>
			<Select.Content>
				<Select.Item value="status" label="Status">Status</Select.Item>
				<Select.Item value="assignee" label="Assignee">Assignee</Select.Item>
				<Select.Item value="priority" label="Priority">Priority</Select.Item>
				<Select.Item value="label" label="Label">Label</Select.Item>
			</Select.Content>
		</Select.Root>
	</div>

	<div class="flex flex-1 gap-3 overflow-x-auto pb-2">
		{#each columns as column (column.key)}
			<div class="flex w-[240px] shrink-0 flex-col sm:w-[260px]">
				<!-- column header -->
				<div class="flex items-center gap-2 px-1 pb-2">
					{#if groupBy === 'status'}
						<StatusIcon status={column.key as TaskStatus} size={14} />
					{:else if groupBy === 'priority'}
						<PriorityIcon priority={Number(column.key) as TaskPriority} size={14} />
					{:else if groupBy === 'label' && column.key}
						{@const color = labelMap.get(column.key)?.color}
						<span
							class="size-2 shrink-0 rounded-full {color
								? labelColorMap[color].dot
								: 'bg-muted-foreground/60'}"
						></span>
					{:else if groupBy === 'assignee' && column.key}
						<span
							class="flex size-4 shrink-0 items-center justify-center rounded-full font-mono text-[9px]"
							style="color: {agentColor(column.key)}; background: {agentColor(column.key)}1a"
							>@</span
						>
					{/if}
					<span class="truncate text-[13px] font-medium text-foreground">{column.title}</span>
					<span class="text-[12px] text-muted-foreground/60">{column.tasks.length}</span>
					<div class="flex-1"></div>
					<Button
						variant="ghost"
						size="icon-xs"
						class="flex size-5 items-center justify-center rounded text-muted-foreground/40 transition-colors hover:bg-muted hover:text-foreground"
						aria-label={`Add task to ${column.title}`}
						onclick={() => (dialogOpen = true)}
					>
						<Icon icon="add" size={14} />
					</Button>
				</div>

				<!-- cards -->
				<div
					class="flex flex-1 flex-col gap-1.5 overflow-y-auto"
					use:dropZone={{
						container: containerFor(column),
						direction: 'vertical',
						onDrop: (state: DragDropState<Task>) => handleDrop(column, state, null)
					}}
				>
					{#if column.tasks.length === 0}
						<div
							class="flex flex-col items-center gap-1 rounded-lg border border-dashed border-border/50 py-8 text-center"
						>
							<span class="text-[12px] text-muted-foreground/40">No tasks</span>
						</div>
					{:else}
						{#each column.tasks as task (task.id)}
							<TaskCard
								{task}
								container={containerFor(column)}
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
								onBoardDrop={(state, t) => handleDrop(column, state, t)}
							/>
						{/each}
					{/if}
				</div>
			</div>
		{/each}
	</div>
</div>
