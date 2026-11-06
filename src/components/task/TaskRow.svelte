<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import PriorityIcon from './PriorityIcon.svelte';
	import PriorityMenu from './PriorityMenu.svelte';
	import StatusMenu from './StatusMenu.svelte';
	import TaskLabels from './TaskLabels.svelte';
	import DueDateBadge from './DueDateBadge.svelte';
	import EndDateBadge from './EndDateBadge.svelte';
	import ReminderBadge from './ReminderBadge.svelte';
	import RecurrenceBadge from './RecurrenceBadge.svelte';
	import TimerBadge from './TimerBadge.svelte';
	import TaskMenu from './TaskMenu.svelte';
	import { sortableItem, type DragDropState } from '$lib/dnd';
	import type { Task, TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { issueId, formatDate } from '$lib/task/utils';
	import { agentColor } from '$lib/agents';
	import Icon from '$lib/components/Icon.svelte';

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
		onListDrop,
		childrenExpanded = false,
		onToggleChildren = undefined
	}: {
		task: Task;
		childrenExpanded?: boolean;
		onToggleChildren?: () => void;
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
		<PriorityMenu value={task.priority} onSelect={(p) => onChangePriority(task, p as TaskPriority)}>
			{#snippet trigger(props)}
				<Button
					{...props}
					variant="ghost"
					size="icon-xs"
					class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/50 transition-colors hover:bg-muted hover:text-muted-foreground"
					aria-label={`Set priority for ${task.title}`}
				>
					<PriorityIcon priority={task.priority} size={14} />
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

		<!-- recurrence indicator -->
		{#if task.recurrenceRule}
			<div class="flex w-5 shrink-0 justify-center">
				<RecurrenceBadge rule={task.recurrenceRule} />
			</div>
		{/if}

		<!-- reminder indicator -->
		{#if task.reminderAt}
			<div class="flex w-5 shrink-0 justify-center">
				<ReminderBadge reminderAt={task.reminderAt} sentAt={task.reminderSentAt} />
			</div>
		{/if}

		<!-- assignee badge -->
		{#if task.assignee}
			<span
				class="shrink-0 rounded-full px-1.5 py-0.5 font-mono text-[10px]"
				style="color: {agentColor(task.assignee)}; background: {agentColor(task.assignee)}1a"
				title="Assignee">@{task.assignee}</span
			>
		{/if}

		<!-- tracked time (badge self-hides) -->
		<TimerBadge {task} />

		<!-- blocked + children -->
		{#if task.blocked || (task.childrenTotal ?? 0) > 0}
			<div class="hidden shrink-0 items-center gap-1.5 md:flex">
				{#if task.blocked}
					<span class="text-red-400" title="Blocked">
						<Icon icon="blocked" size={13} />
					</span>
				{/if}
				{#if (task.childrenTotal ?? 0) > 0}
					<button
						class="flex shrink-0 items-center gap-0.5 rounded-sm text-[11px] text-muted-foreground/50 tabular-nums transition-colors hover:text-foreground"
						title="Toggle child tasks"
						onpointerdown={(e) => e.stopPropagation()}
						onclick={(e) => {
							e.stopPropagation();
							onToggleChildren?.();
						}}
					>
						<Icon
							icon="arrow-down"
							size={10}
							class="transition-transform {childrenExpanded ? '' : '-rotate-90'}"
						/>
						{task.childrenDone ?? 0}/{task.childrenTotal}
					</button>
				{/if}
			</div>
		{/if}

		<!-- pinned indicator -->
		{#if task.pinned}
			<div class="flex w-5 shrink-0 justify-center">
				<Icon icon="pin" size={13} class="text-foreground/60" />
			</div>
		{/if}

		<!-- due date -->
		{#if task.dueDate}
			<div class="hidden shrink-0 justify-end md:flex">
				<DueDateBadge dueDate={task.dueDate} />
			</div>
		{/if}

		<!-- end date -->
		{#if task.endDate}
			<div class="hidden shrink-0 justify-end md:flex">
				<EndDateBadge endDate={task.endDate} />
			</div>
		{/if}

		<!-- date -->
		<span
			class="hidden w-[86px] shrink-0 text-right text-[11px] text-muted-foreground/40 tabular-nums md:block"
		>
			{formatDate(task.updatedAt)}
		</span>
	</article>
</div>
<DropdownMenu.Root bind:open={menuOpen}>
	<DropdownMenu.Trigger
		class="h-0 w-0 outline-none"
		style="position: fixed; left: {menuX}px; top: {menuY}px"
	/>
	<TaskMenu {task} {onEdit} {onTogglePin} {onChangeStatus} {onDuplicate} {onDelete} />
</DropdownMenu.Root>
