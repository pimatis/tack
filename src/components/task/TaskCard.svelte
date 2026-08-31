<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import StatusIcon from '../StatusIcon.svelte';
	import TaskLabels from './TaskLabels.svelte';
	import DueDateBadge from './DueDateBadge.svelte';
	import EndDateBadge from './EndDateBadge.svelte';
	import TaskContextMenu from './TaskContextMenu.svelte';
	import { sortableItem, type DragDropState } from '$lib/dnd';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { statusConfig, statusOrder, priorityConfig } from '$lib/task/constants';
	import { issueId } from '$lib/task/utils';

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
</script>

<ContextMenu.Root>
	<ContextMenu.Trigger class="contents">
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
					<svg
						class="shrink-0 text-foreground"
						width="13"
						height="13"
						viewBox="0 0 24 24"
						fill="none"
					>
						<path
							fill="currentColor"
							d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
						/>
					</svg>
				{/if}
				<div class="flex-1"></div>
				<!-- priority -->
				<Popover.Root>
					<Popover.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon-xs"
								class="flex size-4 shrink-0 items-center justify-center rounded text-muted-foreground/60 transition-colors hover:bg-muted hover:text-foreground"
								aria-label={`Set priority for ${task.title}`}
							>
								{#if task.priority === 1}
									<svg
										class="shrink-0 text-orange-500"
										width="13"
										height="13"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M10.7 3.148a1.5 1.5 0 0 1 2.6 0l8.633 14.954a1.5 1.5 0 0 1-1.299 2.25H3.366a1.5 1.5 0 0 1-1.299-2.25zM12 15.001a1 1 0 1 0 0 2a1 1 0 0 0 0-2m0-7a1 1 0 0 0-1 1v4a1 1 0 0 0 2 0v-4a1 1 0 0 0-1-1"
										/></svg
									>
								{:else if task.priority > 1}
									<svg
										class="shrink-0 text-muted-foreground"
										width="13"
										height="13"
										viewBox="0 0 24 24"
										fill="none"
										><rect
											x="3"
											y="14"
											width="3.5"
											height="7"
											rx="1"
											fill="currentColor"
											opacity={task.priority >= 2 ? 1 : 0.25}
										/><rect
											x="10.25"
											y="9"
											width="3.5"
											height="12"
											rx="1"
											fill="currentColor"
											opacity={task.priority >= 3 ? 1 : 0.25}
										/><rect
											x="17.5"
											y="4"
											width="3.5"
											height="17"
											rx="1"
											fill="currentColor"
											opacity={task.priority >= 4 ? 1 : 0.25}
										/></svg
									>
								{/if}
							</Button>
						{/snippet}
					</Popover.Trigger>
					<Popover.Content class="w-48 p-1.5" align="start">
						<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
							Set priority
						</div>
						{#each [0, 1, 2, 3, 4] as p (p)}
							<Button
								variant="ghost"
								class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
								onclick={() => onChangePriority(task, p as TaskPriority)}
							>
								{#if p === 1}
									<svg
										class="shrink-0 text-orange-500"
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M10.7 3.148a1.5 1.5 0 0 1 2.6 0l8.633 14.954a1.5 1.5 0 0 1-1.299 2.25H3.366a1.5 1.5 0 0 1-1.299-2.25zM12 15.001a1 1 0 1 0 0 2a1 1 0 0 0 0-2m0-7a1 1 0 0 0-1 1v4a1 1 0 0 0 2 0v-4a1 1 0 0 0-1-1"
										/></svg
									>
								{:else}
									<svg class="shrink-0" width="14" height="14" viewBox="0 0 24 24" fill="none">
										<rect
											x="3"
											y="14"
											width="3.5"
											height="7"
											rx="1"
											fill="currentColor"
											opacity={p >= 2 ? 1 : 0.25}
										/>
										<rect
											x="10.25"
											y="9"
											width="3.5"
											height="12"
											rx="1"
											fill="currentColor"
											opacity={p >= 3 ? 1 : 0.25}
										/>
										<rect
											x="17.5"
											y="4"
											width="3.5"
											height="17"
											rx="1"
											fill="currentColor"
											opacity={p >= 4 ? 1 : 0.25}
										/>
									</svg>
								{/if}
								<span>{priorityConfig[p].label}</span>
								{#if task.priority === p}
									<svg
										class="ml-auto text-muted-foreground"
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M21.546 5.111a1.5 1.5 0 0 1 0 2.121L10.303 18.475a1.6 1.6 0 0 1-2.263 0L2.454 12.89a1.5 1.5 0 1 1 2.121-2.121l4.596 4.596L19.424 5.111a1.5 1.5 0 0 1 2.122 0"
										/></svg
									>
								{/if}
							</Button>
						{/each}
					</Popover.Content>
				</Popover.Root>
				<!-- status popover -->
				<Popover.Root>
					<Popover.Trigger>
						{#snippet child({ props })}
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
					</Popover.Trigger>
					<Popover.Content class="w-44 p-1.5" align="start">
						<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
							Change status
						</div>
						{#each statusOrder as s (s)}
							<Button
								variant="ghost"
								class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
								onclick={() => onChangeStatus(task, s)}
							>
								<StatusIcon status={s} size={14} />
								<span>{statusConfig[s].label}</span>
								{#if task.status === s}
									<svg
										class="ml-auto text-muted-foreground"
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M21.546 5.111a1.5 1.5 0 0 1 0 2.121L10.303 18.475a1.6 1.6 0 0 1-2.263 0L2.454 12.89a1.5 1.5 0 1 1 2.121-2.121l4.596 4.596L19.424 5.111a1.5 1.5 0 0 1 2.122 0"
										/></svg
									>
								{/if}
							</Button>
						{/each}
					</Popover.Content>
				</Popover.Root>
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
	</ContextMenu.Trigger>
	<TaskContextMenu {task} {onEdit} {onTogglePin} {onChangeStatus} {onDuplicate} {onDelete} />
</ContextMenu.Root>
