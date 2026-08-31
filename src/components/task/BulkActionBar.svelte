<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import StatusIcon from '../StatusIcon.svelte';
	import type { TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import { statusConfig, priorityConfig } from '$lib/task/constants';

	let {
		selectedCount,
		isAllSelected,
		onToggleSelectAll,
		statusOrder,
		projects,
		onBulkChangeStatus,
		onBulkChangePriority,
		onBulkMoveProject,
		onBulkDuplicate,
		onBulkDelete,
		onClearSelection
	}: {
		selectedCount: number;
		isAllSelected: boolean;
		onToggleSelectAll: () => void;
		statusOrder: TaskStatus[];
		projects: Project[];
		onBulkChangeStatus: (status: TaskStatus) => void;
		onBulkChangePriority: (priority: TaskPriority) => void;
		onBulkMoveProject: (projectId: string) => void;
		onBulkDuplicate: () => void;
		onBulkDelete: () => void;
		onClearSelection: () => void;
	} = $props();
</script>

<div class="flex items-center gap-2 pb-4">
	<Checkbox checked={isAllSelected} onCheckedChange={() => onToggleSelectAll()} />
	<span class="text-[12px] font-medium text-foreground">
		{selectedCount}
		{selectedCount === 1 ? 'task' : 'tasks'} selected
	</span>

	<div class="flex-1"></div>

	<div class="flex items-center gap-1.5">
		<!-- bulk status -->
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
					>
						<span class="size-2 rounded-full bg-muted-foreground/40"></span>
						<span>Status</span>
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-48 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
					Set status for all
				</div>
				{#each statusOrder as s (s)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onBulkChangeStatus(s)}
					>
						<StatusIcon status={s} size={14} />
						<span>{statusConfig[s].label}</span>
					</Button>
				{/each}
			</Popover.Content>
		</Popover.Root>

		<!-- bulk priority -->
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
					>
						<svg width="13" height="13" viewBox="0 0 24 24" fill="none">
							<rect x="3" y="14" width="3.5" height="7" rx="1" fill="currentColor" opacity="0.25" />
							<rect
								x="10.25"
								y="9"
								width="3.5"
								height="12"
								rx="1"
								fill="currentColor"
								opacity="0.25"
							/>
							<rect
								x="17.5"
								y="4"
								width="3.5"
								height="17"
								rx="1"
								fill="currentColor"
								opacity="0.25"
							/>
						</svg>
						<span>Priority</span>
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-48 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
					Set priority for all
				</div>
				{#each [0, 1, 2, 3, 4] as p (p)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onBulkChangePriority(p as TaskPriority)}
					>
						{#if p === 1}
							<svg
								class="shrink-0 text-orange-500"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
							>
								<path
									fill="currentColor"
									d="M10.7 3.148a1.5 1.5 0 0 1 2.6 0l8.633 14.954a1.5 1.5 0 0 1-1.299 2.25H3.366a1.5 1.5 0 0 1-1.299-2.25zM12 15.001a1 1 0 1 0 0 2a1 1 0 0 0 0-2m0-7a1 1 0 0 0-1 1v4a1 1 0 0 0 2 0v-4a1 1 0 0 0-1-1"
								/>
							</svg>
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
					</Button>
				{/each}
			</Popover.Content>
		</Popover.Root>

		<!-- bulk move to project -->
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
					>
						<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M2 5a2 2 0 0 1 2-2h5.52a2 2 0 0 1 1.561.75l1.4 1.75H20a2 2 0 0 1 2 2V19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z"
							/></svg
						>
						<span>Move</span>
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-52 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Move to project</div>
				{#each projects as project (project.id)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onBulkMoveProject(project.id)}
					>
						<span class="shrink-0 font-mono text-[11px] text-muted-foreground/60">
							{project.prefix}
						</span>
						<span class="truncate">{project.name}</span>
					</Button>
				{/each}
			</Popover.Content>
		</Popover.Root>

		<Separator orientation="vertical" class="h-5" />

		<!-- duplicate -->
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-foreground"
						aria-label="Duplicate selected"
						onclick={() => onBulkDuplicate()}
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M9 2a2 2 0 0 0-2 2v2h2V4h11v11h-2v2h2a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM4 7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Duplicate</Tooltip.Content>
		</Tooltip.Root>

		<!-- delete -->
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-destructive"
						aria-label="Delete selected"
						onclick={() => onBulkDelete()}
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Delete</Tooltip.Content>
		</Tooltip.Root>

		<!-- clear selection -->
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-foreground"
						aria-label="Clear selection"
						onclick={() => onClearSelection()}
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Clear selection</Tooltip.Content>
		</Tooltip.Root>
	</div>
</div>
