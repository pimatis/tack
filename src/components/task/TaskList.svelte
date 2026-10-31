<script lang="ts">
	import StatusIcon from './StatusIcon.svelte';
	import TaskRow from './TaskRow.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import type { DragDropState } from '$lib/dnd';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { statusConfig } from '$lib/task/constants';
	import { SvelteSet } from 'svelte/reactivity';

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

	const allTasks = $derived(statusOrderList.flatMap((s) => groups[s]));
	const parentIds = $derived(new Set(allTasks.map((t) => t.id)));
	const childrenOf = $derived.by(() => {
		const map = new Map<string, Task[]>();
		for (const t of allTasks) {
			if (!t.parentId || !parentIds.has(t.parentId)) continue;
			const arr = map.get(t.parentId) ?? [];
			arr.push(t);
			map.set(t.parentId, arr);
		}
		for (const arr of map.values()) arr.sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
		return map;
	});
	const topLevel = (group: Task[]) =>
		group.filter((t) => !t.parentId || !parentIds.has(t.parentId));

	// expanded by default; toggle hides the children tree
	let collapsedTasks = new SvelteSet<string>();
</script>

{#snippet taskRow(task: Task)}
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
		childrenExpanded={!collapsedTasks.has(task.id)}
		onToggleChildren={() =>
			collapsedTasks.has(task.id) ? collapsedTasks.delete(task.id) : collapsedTasks.add(task.id)}
	/>
	{#if !collapsedTasks.has(task.id) && (childrenOf.get(task.id)?.length ?? 0) > 0}
		<div class="mt-0.5 ml-6 grid gap-px pl-2 [&>.tree-item::after]:w-5">
			{#each childrenOf.get(task.id) ?? [] as child (child.id)}
				<div class="tree-item">
					{@render taskRow(child)}
				</div>
			{/each}
		</div>
	{/if}
{/snippet}

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
					<Icon
						icon="arrow-down"
						class="ml-auto size-3.5 text-muted-foreground/50 transition-transform {collapsed
							? '-rotate-90'
							: ''}"
					/>
				</button>

				<!-- task rows -->
				{#if !collapsed}
					<div class="flex flex-col">
						{#each topLevel(group) as task (task.id)}
							{@render taskRow(task)}
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	{/each}
</div>
