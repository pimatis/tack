<script lang="ts">
	import { onMount } from 'svelte';
	import {
		weeklyCounts,
		totals,
		projectCounts,
		trackedPerProject,
		type WeekBucket,
		type StatsTotals,
		type ProjectStat,
		type ProjectTime
	} from '$lib/repositories/stats.repository';
	import { formatSeconds, formatMinutes } from '$lib/task/time';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import Icon from '$lib/components/Icon.svelte';

	let loading = $state(true);
	let weeks = $state<WeekBucket[]>([]);
	let stats = $state<StatsTotals | null>(null);
	let projects = $state<ProjectStat[]>([]);
	let projectTimes = $state<ProjectTime[]>([]);

	onMount(async () => {
		try {
			const [w, t, p, pt] = await Promise.all([
				weeklyCounts(8),
				totals(),
				projectCounts(),
				trackedPerProject()
			]);
			weeks = w;
			stats = t;
			projects = p;
			projectTimes = pt;
		} finally {
			loading = false;
		}
	});

	const maxCount = $derived(Math.max(1, ...weeks.flatMap((w) => [w.created, w.done])));
	const maxTracked = $derived(Math.max(1, ...projectTimes.map((p) => p.seconds)));
	const estimatePct = $derived(
		stats && stats.estimatedMinutes > 0
			? Math.min(100, Math.round((stats.trackedSeconds / (stats.estimatedMinutes * 60)) * 100))
			: 0
	);

	function weekLabel(iso: string) {
		return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric' }).format(
			new Date(iso + 'T00:00:00')
		);
	}
</script>

<svelte:head>
	<title>Stats | Tack</title>
</svelte:head>

<section class="flex h-full flex-col">
	<!-- header -->
	<header
		class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-6 sm:py-4"
	>
		<div class="min-w-0">
			<div class="flex items-center gap-3">
				<h1 class="text-base font-semibold tracking-tight sm:text-lg">Stats</h1>
				{#if !loading && stats}
					<div class="flex items-center gap-1.5 text-[12px] text-muted-foreground">
						<span class="size-1.5 rounded-full bg-foreground/30"></span>
						<span>{stats.open + stats.done + stats.canceled} tasks</span>
					</div>
				{/if}
			</div>
			<p class="truncate text-xs text-muted-foreground sm:text-sm">
				Weekly activity, project progress and tracked time
			</p>
		</div>
		<div class="flex shrink-0 items-center gap-2">
			<Button variant="ghost" size="sm" href="/">
				<Icon icon="arrow-left" size={16} />
				Back
			</Button>
		</div>
	</header>

	<!-- content -->
	<div class="flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
		<div class="mx-auto flex max-w-3xl flex-col gap-6">
			{#if loading}
				<div class="flex items-center justify-center gap-2 py-20 text-[13px] text-muted-foreground">
					<Spinner class="size-3.5" />
					<span>Loading stats...</span>
				</div>
			{:else if !stats}
				<div class="flex flex-col items-center justify-center gap-2 py-20 text-muted-foreground">
					<Icon icon="chart" size={20} class="text-muted-foreground/40" />
					<span class="text-[13px]">No stats yet</span>
					<span class="text-[12px] text-muted-foreground/60">
						Complete tasks to see activity here
					</span>
				</div>
			{:else}
				<!-- totals -->
				<div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
					<div class="rounded-lg border border-border/60 px-3 py-2.5">
						<div class="flex items-center gap-1.5 text-[11px] text-muted-foreground/60">
							<Icon icon="inbox" size={12} />
							<span>Open</span>
						</div>
						<div class="pt-1 text-[20px] font-semibold tabular-nums">{stats.open}</div>
					</div>
					<div class="rounded-lg border border-border/60 px-3 py-2.5">
						<div class="flex items-center gap-1.5 text-[11px] text-muted-foreground/60">
							<Icon icon="check-circle" size={12} />
							<span>Done</span>
						</div>
						<div class="pt-1 text-[20px] font-semibold text-green-500/90 tabular-nums">
							{stats.done}
						</div>
					</div>
					<div class="rounded-lg border border-border/60 px-3 py-2.5">
						<div class="flex items-center gap-1.5 text-[11px] text-muted-foreground/60">
							<Icon icon="alarm" size={12} />
							<span>Overdue</span>
						</div>
						<div
							class="pt-1 text-[20px] font-semibold {stats.overdue > 0
								? 'text-red-400'
								: ''} tabular-nums"
						>
							{stats.overdue}
						</div>
					</div>
					<div class="rounded-lg border border-border/60 px-3 py-2.5">
						<div class="flex items-center gap-1.5 text-[11px] text-muted-foreground/60">
							<Icon icon="stopwatch" size={12} />
							<span>Tracked</span>
							{#if stats.runningTimers > 0}
								<span class="flex items-center gap-1 text-amber-400">
									<span class="size-1 animate-pulse rounded-full bg-amber-400"></span>
									{stats.runningTimers} running
								</span>
							{/if}
						</div>
						<div class="pt-1 text-[20px] font-semibold tabular-nums">
							{formatSeconds(stats.trackedSeconds)}
						</div>
					</div>
				</div>

				<!-- created vs done per week -->
				<div class="rounded-lg border border-border/60 px-4 py-3">
					<div class="flex items-center gap-1.5 pb-3 text-[12px] font-medium text-muted-foreground">
						<Icon icon="chart" size={13} />
						<span>Last 8 weeks</span>
					</div>
					<div class="flex items-end gap-2 sm:gap-3">
						{#each weeks as week (week.weekStart)}
							<div class="flex min-w-0 flex-1 flex-col items-center gap-1.5">
								<div class="flex h-28 w-full items-end justify-center gap-1">
									<Tooltip.Root>
										<Tooltip.Trigger>
											{#snippet child({ props })}
												<div
													{...props}
													class="w-2.5 rounded-t-[3px] bg-muted-foreground/25 sm:w-3"
													style="height: {Math.max(
														week.created > 0 ? 6 : 2,
														(week.created / maxCount) * 100
													)}%"
												></div>
											{/snippet}
										</Tooltip.Trigger>
										<Tooltip.Content sideOffset={6}>
											{week.created} created · week of {weekLabel(week.weekStart)}
										</Tooltip.Content>
									</Tooltip.Root>
									<Tooltip.Root>
										<Tooltip.Trigger>
											{#snippet child({ props })}
												<div
													{...props}
													class="w-2.5 rounded-t-[3px] bg-green-500/70 sm:w-3"
													style="height: {Math.max(
														week.done > 0 ? 6 : 2,
														(week.done / maxCount) * 100
													)}%"
												></div>
											{/snippet}
										</Tooltip.Trigger>
										<Tooltip.Content sideOffset={6}>
											{week.done} done · week of {weekLabel(week.weekStart)}
										</Tooltip.Content>
									</Tooltip.Root>
								</div>
								<span class="truncate text-[10px] text-muted-foreground/50 tabular-nums">
									{weekLabel(week.weekStart)}
								</span>
							</div>
						{/each}
					</div>
					<div class="flex items-center gap-4 pt-3 text-[11px] text-muted-foreground/60">
						<span class="flex items-center gap-1.5">
							<span class="size-2 rounded-sm bg-muted-foreground/25"></span>Created
						</span>
						<span class="flex items-center gap-1.5">
							<span class="size-2 rounded-sm bg-green-500/70"></span>Done
						</span>
					</div>
				</div>

				<!-- estimate vs tracked -->
				{#if stats.estimatedMinutes > 0}
					<div class="rounded-lg border border-border/60 px-4 py-3">
						<div class="flex items-baseline justify-between pb-2">
							<span class="flex items-center gap-1.5 text-[12px] font-medium text-muted-foreground">
								<Icon icon="stopwatch" size={13} />
								<span>Estimates vs tracked</span>
							</span>
							<span class="text-[11px] text-muted-foreground/60 tabular-nums">
								{formatSeconds(stats.trackedSeconds)} of {formatMinutes(stats.estimatedMinutes)}
							</span>
						</div>
						<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
							<div class="h-full rounded-full bg-amber-400/70" style="width: {estimatePct}%"></div>
						</div>
					</div>
				{/if}

				<!-- tracked time per project -->
				{#if projectTimes.length > 0}
					<div class="rounded-lg border border-border/60 px-4 py-3">
						<div
							class="flex items-center gap-1.5 pb-2 text-[12px] font-medium text-muted-foreground"
						>
							<Icon icon="clock" size={13} />
							<span>Time by project</span>
						</div>
						{#each projectTimes as project (project.id)}
							<div class="flex items-center gap-3 py-1.5">
								<span
									class="w-10 shrink-0 truncate rounded border border-border/50 px-1 py-px text-center font-mono text-[10px] text-muted-foreground/60"
								>
									{project.prefix}
								</span>
								<span class="min-w-0 flex-1 truncate text-[13px] text-foreground/90">
									{project.name}
								</span>
								<div class="h-1 w-28 shrink-0 overflow-hidden rounded-full bg-muted">
									<div
										class="h-full rounded-full bg-amber-400/70"
										style="width: {Math.round((project.seconds / maxTracked) * 100)}%"
									></div>
								</div>
								<span
									class="w-16 shrink-0 text-right text-[11px] text-muted-foreground/60 tabular-nums"
								>
									{formatSeconds(project.seconds)}
								</span>
							</div>
						{/each}
					</div>
				{/if}

				<!-- per project -->
				{#if projects.length > 0}
					<div class="rounded-lg border border-border/60 px-4 py-3">
						<div
							class="flex items-center gap-1.5 pb-2 text-[12px] font-medium text-muted-foreground"
						>
							<Icon icon="kanban" size={13} />
							<span>By project</span>
						</div>
						{#each projects as project (project.id)}
							{@const total = project.open + project.done}
							<div class="flex items-center gap-3 py-1.5">
								<span
									class="w-10 shrink-0 truncate rounded border border-border/50 px-1 py-px text-center font-mono text-[10px] text-muted-foreground/60"
								>
									{project.prefix}
								</span>
								<span class="min-w-0 flex-1 truncate text-[13px] text-foreground/90">
									{project.name}
								</span>
								<div class="h-1 w-28 shrink-0 overflow-hidden rounded-full bg-muted">
									<div
										class="h-full rounded-full bg-green-500/70"
										style="width: {total > 0 ? Math.round((project.done / total) * 100) : 0}%"
									></div>
								</div>
								<span
									class="w-16 shrink-0 text-right text-[11px] text-muted-foreground/60 tabular-nums"
								>
									{project.done}/{total} done
								</span>
							</div>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
	</div>
</section>
