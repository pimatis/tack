<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import PriorityIcon from './PriorityIcon.svelte';
	import StatusMenu from './StatusMenu.svelte';
	import PriorityMenu from './PriorityMenu.svelte';
	import type { TaskPriority, TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import { labelColorMap } from '$lib/types/label';
	import type { Task } from '$lib/types/task';
	import Icon from '$lib/components/Icon.svelte';

	let {
		selectedCount,
		isAllSelected,
		onToggleSelectAll,
		projects,
		labels,
		selectedTasks,
		onBulkChangeStatus,
		onBulkChangePriority,
		onBulkMoveProject,
		onBulkLabel,
		onBulkDuplicate,
		onBulkDelete,
		onClearSelection
	}: {
		selectedCount: number;
		isAllSelected: boolean;
		onToggleSelectAll: () => void;
		projects: Project[];
		labels: Label[];
		selectedTasks: Task[];
		onBulkChangeStatus: (status: TaskStatus) => void;
		onBulkChangePriority: (priority: TaskPriority) => void;
		onBulkMoveProject: (projectId: string) => void;
		onBulkLabel: (labelId: string, add: boolean) => void;
		onBulkDuplicate: () => void;
		onBulkDelete: () => void;
		onClearSelection: () => void;
	} = $props();

	// click semantics per label: every selected task not carrying it gets it;
	// when all of them already have it the click removes it from all
	function labelState(labelId: string): 'all' | 'some' | 'none' {
		let count = 0;
		for (const task of selectedTasks) {
			if (task.labelIds?.includes(labelId)) count++;
		}
		return count === 0 ? 'none' : count === selectedTasks.length ? 'all' : 'some';
	}
</script>

<div class="flex flex-wrap items-center gap-2 pb-4">
	<Checkbox checked={isAllSelected} onCheckedChange={() => onToggleSelectAll()} />
	<span class="text-[12px] font-medium text-foreground">
		{selectedCount}
		{selectedCount === 1 ? 'task' : 'tasks'} selected
	</span>

	<div class="flex-1"></div>

	<div class="flex flex-wrap items-center gap-1.5">
		<!-- bulk status -->
		<StatusMenu
			value="todo"
			onSelect={(s) => onBulkChangeStatus(s)}
			title="Set status for all"
			align="end"
		>
			{#snippet trigger(props)}
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
		</StatusMenu>

		<!-- bulk priority -->
		<PriorityMenu
			value={-1}
			onSelect={(p) => onBulkChangePriority(p as TaskPriority)}
			title="Set priority for all"
			align="end"
		>
			{#snippet trigger(props)}
				<Button
					{...props}
					variant="outline"
					size="sm"
					class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
				>
					<PriorityIcon priority={0} size={13} />
					<span>Priority</span>
				</Button>
			{/snippet}
		</PriorityMenu>

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
						<Icon icon="hashtag" size={13} />
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

		<!-- bulk labels -->
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
					>
						<Icon icon="tag" size={13} />
						<span>Labels</span>
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-52 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
					Toggle label for all
				</div>
				{#each labels as label (label.id)}
					{@const state = labelState(label.id)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onBulkLabel(label.id, state !== 'all')}
					>
						<span
							class="size-2.5 shrink-0 rounded-full {labelColorMap[label.color]?.dot ??
								'bg-muted-foreground'}"
						></span>
						<span class="truncate">{label.name}</span>
						{#if state !== 'none'}
							<Icon
								icon="check"
								size={13}
								class="ml-auto shrink-0 {state === 'some' ? 'opacity-40' : ''}"
							/>
						{/if}
					</Button>
				{/each}
				{#if labels.length === 0}
					<div class="px-2 py-1.5 text-[12px] text-muted-foreground">No labels yet</div>
				{/if}
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
						<Icon icon="copy" size={15} />
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
						<Icon icon="trash" size={15} />
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
						<Icon icon="close" size={15} />
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Clear selection</Tooltip.Content>
		</Tooltip.Root>
	</div>
</div>
