<script lang="ts">
	import { onMount } from 'svelte';
	import { fade, fly } from 'svelte/transition';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import StatusIcon from '../StatusIcon.svelte';
	import TaskLabels from './TaskLabels.svelte';
	import DueDateBadge from './DueDateBadge.svelte';
	import TaskContextMenu from './TaskContextMenu.svelte';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';
	import { autofocus } from '$lib/actions/autofocus';
	import PriorityIcon from '../PriorityIcon.svelte';
	import PriorityMenu from '../PriorityMenu.svelte';
	import StatusMenu from '../StatusMenu.svelte';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { issueId } from '$lib/task/utils';

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
</script>

{#if day}
	{@const d = day}
	<div
		class="fixed inset-0 z-40 bg-background/40 backdrop-blur-[2px]"
		transition:fade={{ duration: 200 }}
		onclick={close}
		onkeydown={(e) => {
			if (e.key === 'Escape') close();
		}}
		role="button"
		tabindex="-1"
		aria-label="Close panel"
	></div>

	<div
		class="fixed top-0 right-0 z-50 flex h-screen w-full max-w-[400px] flex-col bg-card shadow-2xl"
		transition:fly={{ x: 400, duration: 280, opacity: 1 }}
		role="dialog"
		aria-modal="true"
		aria-label="Day detail"
		use:autofocus
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape') close();
		}}
	>
		<!-- header -->
		<div class="flex items-start justify-between px-5 pt-5 pb-4">
			<div class="flex flex-col gap-0.5">
				<h2 class="text-[16px] font-semibold tracking-tight text-foreground">
					{formatDay(d)}
				</h2>
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
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
					/></svg
				>
			</Button>
		</div>

		<!-- task list -->
		<div class="flex-1 overflow-y-auto px-3 pb-4">
			{#if sortedTasks.length === 0}
				<div class="flex flex-col items-center gap-2 px-4 py-14 text-center">
					<svg
						class="text-muted-foreground/30"
						width="28"
						height="28"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d="M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-1M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
						/></svg
					>
					<span class="text-[13px] font-medium text-foreground">No tasks on this day</span>
					<span class="text-[12px] text-muted-foreground"> Pick a date to plan something new </span>
				</div>
			{:else}
				<div class="flex flex-col gap-0.5">
					{#each sortedTasks as task (task.id)}
						<ContextMenu.Root>
							<ContextMenu.Trigger class="contents">
								<div
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
												<svg width="9" height="9" viewBox="0 0 24 24" fill="none"
													><path
														fill="currentColor"
														d="M6 3a2 2 0 0 0-2 2v16a1 1 0 1 0 2 0v-5h13.804a1.1 1.1 0 0 0 .89-1.747L17.236 9.5l3.456-4.753A1.1 1.1 0 0 0 19.803 3z"
													/></svg
												>
												{task.endDate.slice(5).replace('-', '/')}
											</span>
										{/if}
									</div>
								</div>
							</ContextMenu.Trigger>
							<TaskContextMenu
								{task}
								{onEdit}
								{onTogglePin}
								{onChangeStatus}
								{onDuplicate}
								{onDelete}
							/>
						</ContextMenu.Root>
					{/each}
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
				<svg class="mr-1" width="12" height="12" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
					/></svg
				>
				New task
			</Button>
		</div>
	</div>
{/if}
