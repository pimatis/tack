<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import type { TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import { labelColorMap } from '$lib/types/label';
	import { statusConfig, statusOrder } from '$lib/task/constants';
	import Icon from '$lib/components/Icon.svelte';

	let {
		searchQuery = $bindable(),
		statusFilters,
		projectFilters,
		labelFilters,
		projects,
		labels,
		hasFilters,
		updatedSinceDays,
		onToggleStatusFilter,
		onToggleProjectFilter,
		onToggleLabelFilter,
		onSetUpdatedSince,
		onClearFilters
	}: {
		searchQuery: string;
		statusFilters: Set<TaskStatus>;
		projectFilters: Set<string>;
		labelFilters: Set<string>;
		projects: Project[];
		labels: Label[];
		hasFilters: boolean;
		updatedSinceDays: number | null;
		onToggleStatusFilter: (status: TaskStatus) => void;
		onToggleProjectFilter: (projectId: string) => void;
		onToggleLabelFilter: (labelId: string) => void;
		onSetUpdatedSince: (days: number | null) => void;
		onClearFilters: () => void;
	} = $props();

	// each filter opens on left-click and on right-click
	let statusOpen = $state(false);
	let projectOpen = $state(false);
	let labelOpen = $state(false);
	let updatedOpen = $state(false);

	const UPDATED_PRESETS: [number, string][] = [
		[1, '24 hours'],
		[7, '7 days'],
		[30, '30 days']
	];

	function openStatus(event: MouseEvent) {
		event.preventDefault();
		statusOpen = true;
	}

	function openProject(event: MouseEvent) {
		event.preventDefault();
		projectOpen = true;
	}

	function openLabel(event: MouseEvent) {
		event.preventDefault();
		labelOpen = true;
	}

	function openUpdated(event: MouseEvent) {
		event.preventDefault();
		updatedOpen = true;
	}
</script>

<div class="flex flex-wrap items-center gap-2 pb-4">
	<!-- search -->
	<div class="relative min-w-0 flex-1 basis-full sm:basis-auto">
		<Icon
			icon="search"
			size={14}
			class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground/50"
		/>
		<Input
			bind:value={searchQuery}
			placeholder="Search tasks..."
			class="h-8 w-full rounded-lg border border-input bg-transparent pr-3 pl-8 text-[13px] text-foreground transition-all outline-none placeholder:text-muted-foreground/50 dark:bg-input/30"
		/>
		{#if searchQuery}
			<Button
				variant="ghost"
				size="icon-xs"
				onclick={() => (searchQuery = '')}
				class="absolute top-1/2 right-2 -translate-y-1/2 text-muted-foreground/50 transition-colors hover:text-foreground"
				aria-label="Clear search"
			>
				<Icon icon="close" size={14} />
			</Button>
		{/if}
	</div>

	<!-- status filter -->
	<DropdownMenu.Root bind:open={statusOpen}>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					oncontextmenu={openStatus}
					variant="outline"
					size="sm"
					class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {statusFilters.size >
					0
						? 'border-border bg-muted/50 text-foreground'
						: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
				>
					<Icon icon="calendar" size={13} />
					<span>Status</span>
					{#if statusFilters.size > 0}
						<span
							class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
							>{statusFilters.size}</span
						>
					{/if}
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-48 p-1.5" align="end">
			<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Filter by status</div>
			{#each statusOrder as s (s)}
				<DropdownMenu.Item
					closeOnSelect={false}
					class="gap-2.5 py-1.5 text-[13px]"
					onclick={() => onToggleStatusFilter(s)}
				>
					<StatusIcon status={s} size={14} />
					<span>{statusConfig[s].label}</span>
					{#if statusFilters.has(s)}
						<Icon icon="check" size={14} class="ml-auto text-muted-foreground" />
					{/if}
				</DropdownMenu.Item>
			{/each}
		</DropdownMenu.Content>
	</DropdownMenu.Root>

	<!-- project filter -->
	<DropdownMenu.Root bind:open={projectOpen}>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					oncontextmenu={openProject}
					variant="outline"
					size="sm"
					class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {projectFilters.size >
					0
						? 'border-border bg-muted/50 text-foreground'
						: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
				>
					<Icon icon="hashtag" size={13} />
					<span>Project</span>
					{#if projectFilters.size > 0}
						<span
							class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
							>{projectFilters.size}</span
						>
					{/if}
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-52 p-1.5" align="end">
			<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Filter by project</div>
			{#if projects.length === 0}
				<div class="px-2 py-3 text-[12px] text-muted-foreground/60">No projects available</div>
			{:else}
				{#each projects as project (project.id)}
					<DropdownMenu.Item
						closeOnSelect={false}
						class="gap-2.5 py-1.5 text-[13px]"
						onclick={() => onToggleProjectFilter(project.id)}
					>
						<span class="shrink-0 font-mono text-[11px] text-muted-foreground/60">
							{project.prefix}
						</span>
						<span class="truncate">{project.name}</span>
						{#if projectFilters.has(project.id)}
							<Icon icon="check" size={14} class="ml-auto shrink-0 text-muted-foreground" />
						{/if}
					</DropdownMenu.Item>
				{/each}
			{/if}
		</DropdownMenu.Content>
	</DropdownMenu.Root>

	<!-- label filter -->
	{#if labels.length > 0}
		<DropdownMenu.Root bind:open={labelOpen}>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						oncontextmenu={openLabel}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {labelFilters.size >
						0
							? 'border-border bg-muted/50 text-foreground'
							: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
					>
						<Icon icon="tag" size={13} />
						<span>Labels</span>
						{#if labelFilters.size > 0}
							<span
								class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
								>{labelFilters.size}</span
							>
						{/if}
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content class="w-52 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Filter by label</div>
				{#each labels as label (label.id)}
					<DropdownMenu.Item
						closeOnSelect={false}
						class="gap-2.5 py-1.5 text-[13px]"
						onclick={() => onToggleLabelFilter(label.id)}
					>
						<span class="size-2.5 shrink-0 rounded-full {labelColorMap[label.color].dot}"></span>
						<span class="truncate">{label.name}</span>
						{#if labelFilters.has(label.id)}
							<Icon icon="check" size={14} class="ml-auto shrink-0 text-muted-foreground" />
						{/if}
					</DropdownMenu.Item>
				{/each}
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	{/if}

	<!-- updated-since filter -->
	<DropdownMenu.Root bind:open={updatedOpen}>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					oncontextmenu={openUpdated}
					variant="outline"
					size="sm"
					class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {updatedSinceDays !==
					null
						? 'border-border bg-muted/50 text-foreground'
						: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
				>
					<Icon icon="clock" size={13} />
					<span>Updated</span>
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-52 p-1.5" align="end">
			<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
				Updated in the last
			</div>
			{#each UPDATED_PRESETS as [days, label] (days)}
				<DropdownMenu.Item
					class="gap-2.5 py-1.5 text-[13px]"
					onclick={() => onSetUpdatedSince(updatedSinceDays === days ? null : days)}
				>
					<span class="truncate">{label}</span>
					{#if updatedSinceDays === days}
						<Icon icon="check" size={14} class="ml-auto shrink-0 text-muted-foreground" />
					{/if}
				</DropdownMenu.Item>
			{/each}
		</DropdownMenu.Content>
	</DropdownMenu.Root>

	{#if hasFilters}
		<Button
			variant="ghost"
			size="sm"
			onclick={onClearFilters}
			class="flex h-8 items-center gap-1 rounded-lg px-2 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
		>
			<Icon icon="close" size={13} />
			<span>Clear</span>
		</Button>
	{/if}
</div>
