<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import TaskLabels from './TaskLabels.svelte';
	import DueDateBadge from './DueDateBadge.svelte';
	import RecurrenceBadge from './RecurrenceBadge.svelte';
	import TaskMenu from './TaskMenu.svelte';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import PriorityIcon from './PriorityIcon.svelte';
	import PriorityMenu from './PriorityMenu.svelte';
	import StatusMenu from './StatusMenu.svelte';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { issueId } from '$lib/task/utils';
	import Icon from '$lib/components/Icon.svelte';

	type Props = {
		day?: Date | null;
		tasks: Task[];
		projects: Project[];
		appSettings: Settings;
		labelMap: Map<string, Label>;
		onEdit: (task: Task) => void;
		onChangeStatus: (task: Task, status: TaskStatus) => void;
		onChangePriority: (task: Task, priority: TaskPriority) => void;
		onTogglePin: (task: Task) => void;
		onDuplicate: (id: string) => void;
		onDelete: (id: string) => void;
		onAddTask: (dueDate: string) => void;
	};

	let {
		day = $bindable(null),
		tasks,
		projects,
		appSettings,
		labelMap,
		onEdit,
		onChangeStatus,
		onChangePriority,
		onTogglePin,
		onDuplicate,
		onDelete,
		onAddTask
	}: Props = $props();

	const open = $derived(day !== null);

	const sortedTasks = $derived(
		[...tasks].sort((a, b) => {
			if (Boolean(a.pinned) !== Boolean(b.pinned)) return a.pinned ? -1 : 1;
			return a.createdAt.localeCompare(b.createdAt);
		})
	);

	function formatDay(date: Date): string {
		return new Intl.DateTimeFormat('en-US', {
			weekday: 'long',
			month: 'long',
			day: 'numeric'
		}).format(date);
	}

	function dayKey(date: Date): string {
		return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
	}

	function isDone(task: Task): boolean {
		return task.status === 'done' || task.status === 'canceled';
	}

	function close() {
		day = null;
	}

	onMount(() => {
		const registry = getShortcutRegistry();
		const unregisterClose = registry.register({
			id: 'close',
			enabled: () => open,
			allowInInput: true,
			run: () => close()
		});
		return () => {
			unregisterClose();
		};
	});

	// right-click on a row opens the dropdown at the pointer
	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);
	let menuTask = $state<Task | null>(null);

	function openMenu(event: MouseEvent, task: Task) {
		event.preventDefault();
		menuTask = task;
		menuX = event.clientX;
		menuY = event.clientY;
		menuOpen = true;
	}
</script>

<Sheet.Root
	{open}
	onOpenChange={(v) => {
		if (!v) close();
	}}
>
	<Sheet.Content
		side="right"
		showCloseButton={false}
		class="w-full! gap-0 bg-card shadow-2xl sm:max-w-[400px]!"
		aria-label="Day detail"
	>
		{#if day}
			{@const d = day}
			<!-- header -->
			<div class="flex items-start justify-between px-5 pt-5 pb-4">
				<div class="flex flex-col gap-0.5">
					<Sheet.Title class="text-[16px] font-semibold tracking-tight">{formatDay(d)}</Sheet.Title>
					<span class="text-[12px] text-muted-foreground">
						{sortedTasks.length === 1 ? '1 task' : `${sortedTasks.length} tasks`}
					</span>
				</div>
				<Button
					variant="ghost"
					size="icon-sm"
					class="text-muted-foreground hover:text-foreground"
					onclick={close}
					aria-label="Close panel"
				>
					<Icon icon="close" size={16} />
				</Button>
			</div>

			<!-- task list -->
			<div class="flex-1 overflow-y-auto px-3 pb-4">
				{#if sortedTasks.length === 0}
					<div class="flex flex-col items-center gap-2 px-4 py-14 text-center">
						<Icon icon="calendar" size={28} class="text-muted-foreground/30" />
						<span class="text-[13px] font-medium text-foreground">No tasks on this day</span>
						<span class="text-[12px] text-muted-foreground">
							Pick a date to plan something new
						</span>
					</div>
				{:else}
					<div class="flex flex-col gap-0.5">
						{#each sortedTasks as task (task.id)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								oncontextmenu={(e) => openMenu(e, task)}
								class="group/row flex w-full cursor-default flex-col gap-1.5 rounded-lg px-2.5 py-2 transition-colors hover:bg-muted/60"
							>
								<div class="flex items-center gap-1.5">
									<!-- status popover -->
									<StatusMenu value={task.status} onSelect={(s) => onChangeStatus(task, s)}>
										{#snippet trigger(props)}
											<Button
												{...props}
												variant="ghost"
												size="icon-xs"
												class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:bg-muted"
												aria-label={`Change status for ${task.title}`}
											>
												<StatusIcon status={task.status} size={14} />
											</Button>
										{/snippet}
									</StatusMenu>

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
												class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground/60 transition-colors hover:bg-muted hover:text-foreground"
												aria-label={`Set priority for ${task.title}`}
											>
												{#if task.priority > 0}
													<PriorityIcon priority={task.priority} size={13} />
												{/if}
											</Button>
										{/snippet}
									</PriorityMenu>

									<!-- title -->
									<button
										type="button"
										class="min-w-0 flex-1 cursor-pointer truncate [mask-image:linear-gradient(to_right,black_95%,transparent_100%)] text-left text-[13px] [-webkit-mask-image:linear-gradient(to_right,black_95%,transparent_100%)] {isDone(
											task
										)
											? 'text-muted-foreground/60 line-through'
											: 'text-foreground'}"
										onclick={() => {
											close();
											onEdit(task);
										}}
									>
										{task.title}
									</button>
									<span class="shrink-0 font-mono text-[10px] text-muted-foreground/40">
										{issueId(task, projects, appSettings)}
									</span>
								</div>
								<div class="flex items-center gap-1.5 pl-6">
									{#if (task.labelIds ?? []).length > 0}
										<TaskLabels labelIds={task.labelIds ?? []} {labelMap} max={2} />
									{/if}
									{#if task.dueDate}
										<DueDateBadge dueDate={task.dueDate} />
									{/if}
									{#if task.endDate}
										<span
											class="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground/70"
										>
											<Icon icon="flag" size={9} />
											{task.endDate.slice(5).replace('-', '/')}
										</span>
									{/if}
									<RecurrenceBadge rule={task.recurrenceRule} size={10} />
								</div>
							</div>
						{/each}
						<DropdownMenu.Root bind:open={menuOpen}>
							<DropdownMenu.Trigger
								class="h-0 w-0 outline-none"
								style="position: fixed; left: {menuX}px; top: {menuY}px"
							/>
							{#if menuTask}
								<TaskMenu
									task={menuTask}
									{onEdit}
									{onTogglePin}
									{onChangeStatus}
									{onDuplicate}
									{onDelete}
								/>
							{/if}
						</DropdownMenu.Root>
					</div>
				{/if}
			</div>

			<!-- footer -->
			<div class="flex items-center justify-between border-t border-border px-5 py-3">
				<span class="text-[11px] text-muted-foreground/60">{dayKey(d)}</span>
				<Button
					size="sm"
					onclick={() => {
						// open the create dialog first, then close the panel so the
						// dialog state survives the panel's re-render
						onAddTask(dayKey(d));
						close();
					}}
				>
					<Icon icon="add" size={12} class="mr-1" />
					New task
				</Button>
			</div>
		{/if}
	</Sheet.Content>
</Sheet.Root>
