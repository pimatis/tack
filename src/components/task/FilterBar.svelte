<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import StatusIcon from '../StatusIcon.svelte';
	import type { TaskStatus } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import { labelColorMap } from '$lib/types/label';
	import { statusConfig, statusOrder } from '$lib/task/constants';

	let {
		searchQuery = $bindable(),
		statusFilters,
		projectFilters,
		labelFilters,
		projects,
		labels,
		hasFilters,
		onToggleStatusFilter,
		onToggleProjectFilter,
		onToggleLabelFilter,
		onClearFilters
	}: {
		searchQuery: string;
		statusFilters: Set<TaskStatus>;
		projectFilters: Set<string>;
		labelFilters: Set<string>;
		projects: Project[];
		labels: Label[];
		hasFilters: boolean;
		onToggleStatusFilter: (status: TaskStatus) => void;
		onToggleProjectFilter: (projectId: string) => void;
		onToggleLabelFilter: (labelId: string) => void;
		onClearFilters: () => void;
	} = $props();
</script>

<div class="flex flex-wrap items-center gap-2 pb-4">
	<!-- search -->
	<div class="relative min-w-0 flex-1 basis-full sm:basis-auto">
		<svg
			class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-muted-foreground/50"
			width="14"
			height="14"
			viewBox="0 0 24 24"
			fill="none"
			><path
				fill="currentColor"
				d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
			/></svg
		>
		<Input
			bind:value={searchQuery}
			placeholder="Filter by title..."
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
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
					/></svg
				>
			</Button>
		{/if}
	</div>

	<!-- status filter -->
	<Popover.Root>
		<Popover.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					variant="outline"
					size="sm"
					class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {statusFilters.size >
					0
						? 'border-border bg-muted/50 text-foreground'
						: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
				>
					<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-1M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
						/></svg
					>
					<span>Status</span>
					{#if statusFilters.size > 0}
						<span
							class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
							>{statusFilters.size}</span
						>
					{/if}
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Content class="w-48 p-1.5" align="end">
			<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Filter by status</div>
			{#each statusOrder as s (s)}
				<Button
					variant="ghost"
					class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
					onclick={() => onToggleStatusFilter(s)}
				>
					<StatusIcon status={s} size={14} />
					<span>{statusConfig[s].label}</span>
					{#if statusFilters.has(s)}
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

	<!-- project filter -->
	<Popover.Root>
		<Popover.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					variant="outline"
					size="sm"
					class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {projectFilters.size >
					0
						? 'border-border bg-muted/50 text-foreground'
						: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
				>
					<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M2 5a2 2 0 0 1 2-2h5.52a2 2 0 0 1 1.561.75l1.4 1.75H20a2 2 0 0 1 2 2V19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2z"
						/></svg
					>
					<span>Project</span>
					{#if projectFilters.size > 0}
						<span
							class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
							>{projectFilters.size}</span
						>
					{/if}
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Content class="w-52 p-1.5" align="end">
			<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Filter by project</div>
			{#if projects.length === 0}
				<div class="px-2 py-3 text-[12px] text-muted-foreground/60">No projects available</div>
			{:else}
				{#each projects as project (project.id)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onToggleProjectFilter(project.id)}
					>
						<span class="shrink-0 font-mono text-[11px] text-muted-foreground/60">
							{project.prefix}
						</span>
						<span class="truncate">{project.name}</span>
						{#if projectFilters.has(project.id)}
							<svg
								class="ml-auto shrink-0 text-muted-foreground"
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
			{/if}
		</Popover.Content>
	</Popover.Root>

	<!-- label filter -->
	{#if labels.length > 0}
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border px-2.5 text-[12px] font-medium transition-colors {labelFilters.size >
						0
							? 'border-border bg-muted/50 text-foreground'
							: 'border-input text-muted-foreground hover:bg-muted/30 hover:text-foreground'}"
					>
						<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M10.537 2.164a3 3 0 0 1 2.244.727l.15.14 7.822 7.823a3 3 0 0 1 .135 4.098l-.135.144-5.657 5.657a3 3 0 0 1-4.098.135l-.144-.135L3.03 12.93a3 3 0 0 1-.878-2.188l.011-.205.472-5.185a3 3 0 0 1 2.537-2.695l.179-.021zM8.024 8.025a2 2 0 1 0 2.829 2.829 2 2 0 0 0-2.829-2.829"
							/></svg
						>
						<span>Labels</span>
						{#if labelFilters.size > 0}
							<span
								class="flex size-4 items-center justify-center rounded-full bg-foreground/10 text-[10px] font-semibold"
								>{labelFilters.size}</span
							>
						{/if}
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-52 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Filter by label</div>
				{#each labels as label (label.id)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onToggleLabelFilter(label.id)}
					>
						<span class="size-2.5 shrink-0 rounded-full {labelColorMap[label.color].dot}"></span>
						<span class="truncate">{label.name}</span>
						{#if labelFilters.has(label.id)}
							<svg
								class="ml-auto shrink-0 text-muted-foreground"
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
	{/if}

	{#if hasFilters}
		<Button
			variant="ghost"
			size="sm"
			onclick={onClearFilters}
			class="flex h-8 items-center gap-1 rounded-lg px-2 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
		>
			<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
				/></svg
			>
			<span>Clear</span>
		</Button>
	{/if}
</div>
