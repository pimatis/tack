<script lang="ts">
	import { onMount } from 'svelte';
	import type { Task } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { labelColorMap } from '$lib/types/label';
	import { issueId } from '$lib/task/utils';
	import StatusIcon from './StatusIcon.svelte';

	type Props = {
		tasks: Task[];
		projects: Project[];
		appSettings: Settings;
		labelMap: Map<string, Label>;
		onEdit: (task: Task) => void;
	};

	let { tasks, projects, appSettings, labelMap, onEdit }: Props = $props();

	const DAY_W = 36;
	const LABEL_W = 220;
	const ROW_H = 32;
	const PAD_DAYS = 2;

	let scroller = $state<HTMLDivElement | null>(null);
	let scrollLeft = $state(0);

	function toDate(value: string): Date {
		const [y, m, d] = value.split('-').map(Number);
		return new Date(y, m - 1, d);
	}

	function dayKey(date: Date): string {
		return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
	}

	function addDays(date: Date, n: number): Date {
		const d = new Date(date);
		d.setDate(d.getDate() + n);
		return d;
	}

	function todayStart(): Date {
		const n = new Date();
		return new Date(n.getFullYear(), n.getMonth(), n.getDate());
	}

	const rows = $derived.by(() => {
		type Span = { task: Task; start: Date; end: Date };
		const dated: Span[] = [];
		const undated: Task[] = [];

		for (const task of tasks) {
			let start = task.dueDate ? toDate(task.dueDate) : null;
			let end = task.endDate ? toDate(task.endDate) : null;
			if (start && end && end < start) [start, end] = [end, start];
			if (!start && !end) {
				undated.push(task);
				continue;
			}
			dated.push({ task, start: (start ?? end)!, end: (end ?? start)! });
		}
		dated.sort(
			(a, b) => a.start.getTime() - b.start.getTime() || a.end.getTime() - b.end.getTime()
		);

		// the window covers every span plus today, padded on both ends
		const today = todayStart();
		let rangeStart = addDays(today, -14);
		let rangeEnd = addDays(today, 14);
		for (const d of dated) {
			const lo = addDays(d.start, -PAD_DAYS);
			const hi = addDays(d.end, PAD_DAYS);
			if (lo < rangeStart) rangeStart = lo;
			if (hi > rangeEnd) rangeEnd = hi;
		}

		const days: Date[] = [];
		for (let d = rangeStart; d <= rangeEnd; d = addDays(d, 1)) days.push(d);

		const index = new Map(days.map((d, i) => [dayKey(d), i]));
		const bars = dated.map((d) => ({
			task: d.task,
			startIdx: index.get(dayKey(d.start)) ?? 0,
			endIdx: index.get(dayKey(d.end)) ?? days.length - 1
		}));

		return { days, bars, undated };
	});

	const monthFmt = new Intl.DateTimeFormat('en-US', { month: 'short', year: 'numeric' });

	const monthSegments = $derived.by(() => {
		const fmt = monthFmt;
		const segs: { label: string; span: number }[] = [];
		for (const d of rows.days) {
			const label = fmt.format(d);
			const last = segs[segs.length - 1];
			if (last && last.label === label) last.span += 1;
			else segs.push({ label, span: 1 });
		}
		return segs;
	});

	const todayIdx = $derived(rows.days.findIndex((d) => dayKey(d) === dayKey(todayStart())));

	// month of the first visible day, pinned in the sticky corner cell
	const visibleMonth = $derived.by(() => {
		const idx = Math.min(Math.max(Math.floor(scrollLeft / DAY_W), 0), rows.days.length - 1);
		return rows.days[idx] ? monthFmt.format(rows.days[idx]) : '';
	});

	const WEEKDAY_LETTERS = ['S', 'M', 'T', 'W', 'T', 'F', 'S'] as const;

	function isWeekend(date: Date): boolean {
		return date.getDay() === 0 || date.getDay() === 6;
	}

	function barClass(task: Task): string {
		if (task.status === 'done' || task.status === 'canceled') return 'bg-muted-foreground/25';
		const end = task.endDate ?? task.dueDate;
		if (end && toDate(end).getTime() < todayStart().getTime()) return 'bg-destructive/60';
		const label = task.labelIds?.[0] ? labelMap.get(task.labelIds[0]) : undefined;
		return label ? labelColorMap[label.color].dot : 'bg-primary/70';
	}

	function taskTitle(task: Task): string {
		return `${issueId(task, projects, appSettings)} · ${task.title}`;
	}

	onMount(() => {
		if (!scroller || todayIdx < 0) return;
		scroller.scrollLeft = Math.max(0, todayIdx * DAY_W - 3 * DAY_W);
	});
</script>

<div
	class="h-full overflow-auto"
	bind:this={scroller}
	onscroll={(e) => (scrollLeft = e.currentTarget.scrollLeft)}
>
	<div class="relative" style="min-width: {LABEL_W + rows.days.length * DAY_W}px">
		<!-- header: months over day cells, sticky top -->
		<div class="sticky top-0 z-30 flex bg-background">
			<div
				class="sticky left-0 z-10 flex shrink-0 items-center justify-center border-r border-b border-border/60 bg-background text-[11px] font-medium text-muted-foreground/70"
				style="width: {LABEL_W}px"
			>
				{visibleMonth}
			</div>
			<div class="border-b border-border/60">
				<div class="flex">
					{#each monthSegments as seg (seg.label + seg.span)}
						<div
							class="shrink-0 truncate border-l border-border/40 px-1.5 pt-1 text-[10px] font-medium text-muted-foreground/70"
							style="width: {seg.span * DAY_W}px"
						>
							{seg.label}
						</div>
					{/each}
				</div>
				<div class="flex">
					{#each rows.days as day, i (dayKey(day))}
						<div
							class="flex shrink-0 flex-col items-center border-l border-border/40 py-0.5 {i ===
							todayIdx
								? 'bg-primary/10 text-primary'
								: isWeekend(day)
									? 'bg-muted/30 text-muted-foreground/50'
									: 'text-muted-foreground'}"
							style="width: {DAY_W}px"
						>
							<span class="text-[9px] leading-tight">{WEEKDAY_LETTERS[day.getDay()]}</span>
							<span class="text-[11px] leading-tight tabular-nums">{day.getDate()}</span>
						</div>
					{/each}
				</div>
			</div>
		</div>

		<!-- body: one grid background under every row -->
		<div class="relative">
			<div class="absolute inset-y-0 flex" style="left: {LABEL_W}px">
				{#each rows.days as day, i (dayKey(day))}
					<div
						class="h-full shrink-0 border-l border-border/40 {i === todayIdx
							? 'bg-primary/5'
							: isWeekend(day)
								? 'bg-muted/20'
								: ''}"
						style="width: {DAY_W}px"
					></div>
				{/each}
			</div>
			{#if todayIdx >= 0}
				<div
					class="pointer-events-none absolute inset-y-0 z-10 w-px bg-primary/60"
					style="left: {LABEL_W + todayIdx * DAY_W + DAY_W / 2}px"
				></div>
			{/if}

			{#each rows.bars as bar (bar.task.id)}
				{@const isDone = bar.task.status === 'done' || bar.task.status === 'canceled'}
				<div class="relative flex items-center border-b border-border/40" style="height: {ROW_H}px">
					<button
						type="button"
						class="sticky left-0 z-20 flex h-full shrink-0 cursor-pointer items-center gap-2 border-r border-border/60 bg-background px-2.5 transition-colors hover:bg-muted/40"
						style="width: {LABEL_W}px"
						onclick={() => onEdit(bar.task)}
						title={taskTitle(bar.task)}
					>
						<StatusIcon status={bar.task.status} size={13} />
						<span class="shrink-0 font-mono text-[11px] text-muted-foreground/50">
							{issueId(bar.task, projects, appSettings)}
						</span>
						<span
							class="min-w-0 truncate text-[13px] text-foreground/90 {isDone
								? 'line-through opacity-60'
								: ''}"
						>
							{bar.task.title}
						</span>
					</button>
					<button
						type="button"
						class="absolute z-10 h-5 cursor-pointer rounded-md transition-opacity hover:opacity-80 {barClass(
							bar.task
						)}"
						style="left: {LABEL_W + bar.startIdx * DAY_W + 3}px; width: {Math.max(
							(bar.endIdx - bar.startIdx + 1) * DAY_W - 6,
							8
						)}px"
						onclick={() => onEdit(bar.task)}
						title={taskTitle(bar.task)}
						aria-label={taskTitle(bar.task)}
					></button>
				</div>
			{/each}

			{#if rows.undated.length > 0}
				<div
					class="relative flex items-center border-b border-border/60 bg-muted/30"
					style="height: {ROW_H}px"
				>
					<div
						class="sticky left-0 z-20 flex h-full shrink-0 items-center border-r border-border/60 bg-muted/30 px-2.5 text-[11px] font-medium text-muted-foreground/60"
						style="width: {LABEL_W}px"
					>
						No dates · {rows.undated.length}
					</div>
				</div>
				{#each rows.undated as task (task.id)}
					<div
						class="relative flex items-center border-b border-border/40"
						style="height: {ROW_H}px"
					>
						<button
							type="button"
							class="sticky left-0 z-20 flex h-full shrink-0 cursor-pointer items-center gap-2 border-r border-border/60 bg-background px-2.5 transition-colors hover:bg-muted/40"
							style="width: {LABEL_W}px"
							onclick={() => onEdit(task)}
							title={taskTitle(task)}
						>
							<StatusIcon status={task.status} size={13} />
							<span class="shrink-0 font-mono text-[11px] text-muted-foreground/50">
								{issueId(task, projects, appSettings)}
							</span>
							<span
								class="min-w-0 truncate text-[13px] text-foreground/90 {task.status === 'done' ||
								task.status === 'canceled'
									? 'line-through opacity-60'
									: ''}"
							>
								{task.title}
							</span>
						</button>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
