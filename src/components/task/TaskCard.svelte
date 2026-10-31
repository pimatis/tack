<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import TaskLabels from './TaskLabels.svelte';
	import DueDateBadge from './DueDateBadge.svelte';
	import EndDateBadge from './EndDateBadge.svelte';
	import ReminderBadge from './ReminderBadge.svelte';
	import RecurrenceBadge from './RecurrenceBadge.svelte';
	import TimerBadge from './TimerBadge.svelte';
	import TaskMenu from './TaskMenu.svelte';
	import { sortableItem, type DragDropState } from '$lib/dnd';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { issueId } from '$lib/task/utils';
	import PriorityIcon from './PriorityIcon.svelte';
	import PriorityMenu from './PriorityMenu.svelte';
	import StatusMenu from './StatusMenu.svelte';
	import Icon from '$lib/components/Icon.svelte';

	let {
		task,
		projects,
		appSettings,
		labelMap,
		selectedIds,
		onEdit,
		onChangeStatus,
		onChangePriority,
		onTogglePin,
		onDuplicate,
		onDelete,
		onBoardDrop
	}: {
		task: Task;
		projects: Project[];
		appSettings: Settings;
		labelMap: Map<string, Label>;
		selectedIds: Set<string>;
		onEdit: (task: Task) => void;
		onChangeStatus: (task: Task, status: TaskStatus) => void;
		onChangePriority: (task: Task, priority: TaskPriority) => void;
		onTogglePin: (task: Task) => void;
		onDuplicate: (id: string) => void;
		onDelete: (id: string) => void;
		onBoardDrop: (state: DragDropState<Task>, targetTask: Task | null) => void;
	} = $props();

	// right-click opens the dropdown at the pointer; left-click keeps its drag/edit behavior
	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);

	function openMenu(event: MouseEvent) {
		event.preventDefault();
		menuX = event.clientX;
		menuY = event.clientY;
		menuOpen = true;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="contents" oncontextmenu={openMenu}>
	<article
		use:sortableItem={{
			dragData: task,
			container: `board-${task.status}`,
			onDrop: (state: DragDropState<Task>) => onBoardDrop(state, task)
		}}
		class="group/card flex cursor-grab flex-col gap-2 rounded-lg border border-border bg-muted/20 p-2.5 transition-all hover:border-border/60 hover:bg-muted/40 active:cursor-grabbing {selectedIds.has(
			task.id
		)
			? 'bg-primary/5'
			: ''}"
	>
		<!-- top row: issue id + actions -->
		<div class="flex items-center gap-1.5">
			<span class="shrink-0 font-mono text-[11px] text-muted-foreground/50">
				{issueId(task, projects, appSettings)}
			</span>
			{#if task.pinned}
				<Icon icon="pin" size={13} class="shrink-0 text-foreground" />
			{/if}
			<ReminderBadge reminderAt={task.reminderAt} sentAt={task.reminderSentAt} />
			<RecurrenceBadge rule={task.recurrenceRule} />
			<TimerBadge {task} />
			{#if task.blocked}
				<span class="text-red-400" title="Blocked">
					<Icon icon="blocked" size={12} />
				</span>
			{/if}
			{#if (task.childrenTotal ?? 0) > 0}
				<span
					class="flex shrink-0 items-center gap-1 text-[11px] text-muted-foreground/50 tabular-nums"
					title="Child tasks"
				>
					<Icon icon="list" size={11} />{task.childrenDone ?? 0}/{task.childrenTotal}
				</span>
			{/if}
			<div class="flex-1"></div>
			<!-- priority -->
			<PriorityMenu
				value={task.priority}
				onSelect={(p) => onChangePriority(task, p as TaskPriority)}
			>
				{#snippet trigger(props)}
					<Button
						{...props}
						variant="ghost"
						size="icon-xs"
						class="flex size-4 shrink-0 items-center justify-center rounded text-muted-foreground/60 transition-colors hover:bg-muted hover:text-foreground"
						aria-label={`Set priority for ${task.title}`}
					>
						<PriorityIcon priority={task.priority} size={13} />
					</Button>
				{/snippet}
			</PriorityMenu>
			<!-- status popover -->
			<StatusMenu value={task.status} onSelect={(s) => onChangeStatus(task, s)}>
				{#snippet trigger(props)}
					<Button
						{...props}
						variant="ghost"
						size="icon-xs"
						class="flex size-4 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted"
						aria-label={`Change status for ${task.title}`}
					>
						<StatusIcon status={task.status} size={13} />
					</Button>
				{/snippet}
			</StatusMenu>
		</div>

		<!-- title -->
		<Button
			variant="ghost"
			class="h-auto w-full justify-start truncate [mask-image:linear-gradient(to_right,black_85%,transparent_100%)] p-0 text-left text-[13px] text-foreground/90 [-webkit-mask-image:linear-gradient(to_right,black_85%,transparent_100%)] {task.status ===
			'canceled'
				? 'text-muted-foreground/50 line-through'
				: ''}"
			onclick={() => onEdit(task)}
		>
			{task.title}
		</Button>

		<!-- bottom row: labels + due date -->
		<div class="flex flex-wrap items-center gap-1.5">
			<TaskLabels labelIds={task.labelIds ?? []} {labelMap} max={2} />
			<div class="ml-auto shrink-0">
				<DueDateBadge dueDate={task.dueDate} />
			</div>
			<div class="shrink-0">
				<EndDateBadge endDate={task.endDate} />
			</div>
		</div>
	</article>
</div>
<DropdownMenu.Root bind:open={menuOpen}>
	<DropdownMenu.Trigger
		class="h-0 w-0 outline-none"
		style="position: fixed; left: {menuX}px; top: {menuY}px"
	/>
	<TaskMenu {task} {onEdit} {onTogglePin} {onChangeStatus} {onDuplicate} {onDelete} />
</DropdownMenu.Root>
