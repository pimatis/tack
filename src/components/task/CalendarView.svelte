<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import type { Task, TaskStatus, TaskPriority } from '$lib/types/task';
	import type { Project } from '$lib/types/project';
	import type { Label } from '$lib/types/label';
	import type { Settings } from '$lib/types/settings';
	import { labelColorMap } from '$lib/types/label';
	import { issueId } from '$lib/task/utils';
	import CalendarDayPanel from './CalendarDayPanel.svelte';

	type Props = {
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

	// month currently shown (first day of the month)
	let month = $state(new Date(new Date().getFullYear(), new Date().getMonth(), 1));
	let selectedDay = $state<Date | null>(null);

	const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as const;

	const LANE_HEIGHT = 25;
	const MORE_THRESHOLD = 3;

	type PlacedTask = {
		task: Task;
		startIdx: number;
		endIdx: number;
		lane: number;
	};

	const gridDays = $derived.by(() => {
		const first = new Date(month.getFullYear(), month.getMonth(), 1);
		const start = new Date(first);
		// monday-first week
		start.setDate(first.getDate() - ((first.getDay() + 6) % 7));
		const days: Date[] = [];
		for (let i = 0; i < 42; i++) {
			const d = new Date(start);
			d.setDate(start.getDate() + i);
			days.push(d);
		}
		return days;
	});

	function toDate(value: string): Date {
		const [y, m, d] = value.split('-').map(Number);
		return new Date(y, m - 1, d);
	}

	function dayKey(date: Date): string {
		return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
	}

	const dayIndexMap = $derived(new Map(gridDays.map((d, i) => [dayKey(d), i])));

	function gridIndex(date: Date): number {
		return dayIndexMap.get(dayKey(date)) ?? 0;
	}

	// place bars into lanes so overlapping spans never share a lane
	const placedTasks = $derived.by(() => {
		const windowStart = gridDays[0];
		const windowEnd = gridDays[41];
		const items: { task: Task; start: Date; end: Date }[] = [];

		for (const task of tasks) {
			let start = task.dueDate ? toDate(task.dueDate) : null;
			let end = task.endDate ? toDate(task.endDate) : null;
			if (!start && !end) {
				// no dates set: surface on the day the task was created
				const created = new Date(task.createdAt);
				start = new Date(created.getFullYear(), created.getMonth(), created.getDate());
			}
			if (!start) start = end; // end-only task surfaces on its end day
			if (!end) end = start;
			if (end! < start!) [start, end] = [end, start];
			if (end! < windowStart || start! > windowEnd) continue;
			items.push({ task, start: start!, end: end! });
		}

		items.sort(
			(a, b) => a.start.getTime() - b.start.getTime() || a.end.getTime() - b.end.getTime()
		);

		const laneEnds: number[] = [];
		const placed: PlacedTask[] = items.map((item) => {
			let lane = laneEnds.findIndex((end) => end < item.start.getTime());
			if (lane === -1) {
				lane = laneEnds.length;
				laneEnds.push(0);
			}
			laneEnds[lane] = item.end.getTime();
			return {
				task: item.task,
				startIdx: gridIndex(item.start),
				endIdx: gridIndex(item.end),
				lane
			};
		});

		return placed;
	});

	function activeBars(dayIndex: number): PlacedTask[] {
		return placedTasks.filter((p) => p.startIdx <= dayIndex && p.endIdx >= dayIndex);
	}

	function barsHeight(bars: PlacedTask[]): number {
		const maxLane = bars.reduce((max, b) => Math.max(max, b.lane), -1);
		return (maxLane + 1) * LANE_HEIGHT;
	}

	function taskDotColor(task: Task): string {
		const labelId = task.labelIds?.[0];
		const label = labelId ? labelMap.get(labelId) : undefined;
		return label ? labelColorMap[label.color].dot : 'bg-primary/70';
	}

	function formatMonth(date: Date): string {
		return new Intl.DateTimeFormat('en-US', { month: 'long', year: 'numeric' }).format(date);
	}

	function isToday(date: Date): boolean {
		return dayKey(date) === dayKey(new Date());
	}

	function inCurrentMonth(date: Date): boolean {
		return date.getMonth() === month.getMonth() && date.getFullYear() === month.getFullYear();
	}

	function shiftMonth(delta: number) {
		month = new Date(month.getFullYear(), month.getMonth() + delta, 1);
	}

	function taskTitle(task: Task): string {
		return `${issueId(task, projects, appSettings)} · ${task.title}`;
	}
</script>

<div class="flex h-full flex-col gap-3">
	<!-- month navigation -->
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-1.5">
			<span class="text-[16px] font-semibold tracking-tight text-foreground">
				{formatMonth(month)}
			</span>
			<div class="ml-2 flex items-center gap-0.5">
				<Button
					variant="ghost"
					size="icon-xs"
					class="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
					onclick={() => shiftMonth(-1)}
					aria-label="Previous month"
				>
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M14.06 6.94a1.5 1.5 0 0 1 0 2.12L10.12 13l3.94 3.94a1.5 1.5 0 0 1-2.12 2.12l-5-5a1.5 1.5 0 0 1 0-2.12l5-5a1.5 1.5 0 0 1 2.12 0"
						/></svg
					>
				</Button>
				<Button
					variant="ghost"
					size="icon-xs"
					class="flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
					onclick={() => shiftMonth(1)}
					aria-label="Next month"
				>
					<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M9.94 6.94a1.5 1.5 0 0 0 0 2.12L13.88 13l-3.94 3.94a1.5 1.5 0 0 0 2.12 2.12l5-5a1.5 1.5 0 0 0 0-2.12l-5-5a1.5 1.5 0 0 0-2.12 0"
						/></svg
					>
				</Button>
			</div>
		</div>
	</div>

	<!-- weekday header -->
	<div class="grid grid-cols-7 border-b border-border/60 pb-2">
		{#each WEEKDAYS as day (day)}
			<div class="px-1.5 text-[11px] font-medium text-muted-foreground/70">{day}</div>
		{/each}
	</div>

	<!-- day grid: bordered cells so bars flow seamlessly between days -->
	<div
		class="grid flex-1 grid-cols-7 grid-rows-6 overflow-hidden rounded-lg border border-border/60"
	>
		{#each gridDays as day, i (dayKey(day))}
			{@const bars = activeBars(i)}
			<div
				class="group relative flex min-h-0 cursor-pointer flex-col overflow-hidden border-r border-b border-border/40 p-1.5 transition-colors last:border-r-0 hover:bg-muted/20 {i >=
				35
					? 'border-b-0'
					: ''} {i % 7 === 6 ? 'border-r-0' : ''} {!inCurrentMonth(day) ? 'bg-muted/10' : ''}"
				role="button"
				tabindex="0"
				onclick={() => (selectedDay = day)}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						selectedDay = day;
					}
				}}
			>
				<div class="flex h-6 shrink-0 items-center justify-between pr-0.5">
					<span
						class="flex size-5.5 items-center justify-center rounded-full text-[11px] {isToday(day)
							? 'bg-primary font-semibold text-primary-foreground'
							: inCurrentMonth(day)
								? 'text-foreground'
								: 'text-muted-foreground/40'}"
					>
						{day.getDate()}
					</span>
					<button
						type="button"
						class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-muted-foreground/40 opacity-0 transition-opacity group-hover:opacity-100 hover:bg-muted hover:text-foreground"
						aria-label="Add task on this day"
						onclick={(e) => {
							e.stopPropagation();
							onAddTask(dayKey(day));
						}}
					>
						<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
							/></svg
						>
					</button>
				</div>

				<!-- bars: every task is rendered; the area scrolls when the
				     cell is too small, so nothing ever leaks into the row below -->
				<div
					class="relative min-h-0 flex-1 overflow-y-auto {bars.length > MORE_THRESHOLD
						? '[mask-image:linear-gradient(to_bottom,black_72%,transparent_100%)] [-webkit-mask-image:linear-gradient(to_bottom,black_72%,transparent_100%)]'
						: ''}"
				>
					<div class="relative" style="height: {barsHeight(bars)}px">
						{#each bars as bar (bar.task.id)}
							{@const isStart = bar.startIdx === i}
							{@const isEnd = bar.endIdx === i}
							{@const isDone = bar.task.status === 'done' || bar.task.status === 'canceled'}
							{@const dotColor = taskDotColor(bar.task)}

							<!-- start day: chip with label dot + arrow toward the end -->
							{#if isStart}
								<button
									type="button"
									class="absolute inset-x-1 flex h-5 cursor-pointer items-center gap-1.5 overflow-hidden rounded-md border border-border/50 bg-muted/40 px-1.5 text-left transition-colors hover:bg-muted/70 {isDone
										? 'opacity-50'
										: ''}"
									style="top: {bar.lane * LANE_HEIGHT}px"
									onclick={(e) => {
										e.stopPropagation();
										selectedDay = null;
										onEdit(bar.task);
									}}
									title={taskTitle(bar.task)}
								>
									<span class="size-1.5 shrink-0 rounded-full {dotColor}"></span>
									<span
										class="min-w-0 flex-1 truncate [mask-image:linear-gradient(to_right,black_85%,transparent_100%)] text-[11px] leading-none [-webkit-mask-image:linear-gradient(to_right,black_85%,transparent_100%)] {isDone
											? 'line-through'
											: ''}"
									>
										{bar.task.title}
									</span>
									{#if !isEnd}
										<svg
											class="shrink-0 text-muted-foreground/60"
											width="10"
											height="10"
											viewBox="0 0 24 24"
											fill="none"
											><path
												fill="currentColor"
												d="M9.94 6.94a1.5 1.5 0 0 0 0 2.12L13.88 13l-3.94 3.94a1.5 1.5 0 0 0 2.12 2.12l5-5a1.5 1.5 0 0 0 0-2.12l-5-5a1.5 1.5 0 0 0-2.12 0"
											/></svg
										>
									{/if}
								</button>
							{:else}
								<!-- continuation: dashed primary line toward the end date -->
								<div
									class="absolute inset-x-1.5 flex items-center border-t-2 border-dashed {isDone
										? 'border-primary/15'
										: 'border-primary/35'}"
									style="top: {bar.lane * LANE_HEIGHT + 9}px"
								>
									{#if isEnd}
										<svg
											class="absolute right-0 -translate-y-[6px] text-primary/50"
											width="9"
											height="9"
											viewBox="0 0 24 24"
											fill="none"
											><path
												fill="currentColor"
												d="M9.94 6.94a1.5 1.5 0 0 0 0 2.12L13.88 13l-3.94 3.94a1.5 1.5 0 0 0 2.12 2.12l5-5a1.5 1.5 0 0 0 0-2.12l-5-5a1.5 1.5 0 0 0-2.12 0"
											/></svg
										>
									{/if}
								</div>
							{/if}
						{/each}
					</div>
				</div>

				{#if bars.length > MORE_THRESHOLD}
					<button
						type="button"
						class="shrink-0 cursor-pointer px-1 text-[10px] text-muted-foreground/60 transition-colors hover:text-foreground"
						onclick={(e) => {
							e.stopPropagation();
							selectedDay = day;
						}}
					>
						+{bars.length - MORE_THRESHOLD} more
					</button>
				{/if}
			</div>
		{/each}
	</div>
</div>

<CalendarDayPanel
	bind:day={selectedDay}
	tasks={selectedDay ? activeBars(gridIndex(selectedDay)).map((b) => b.task) : []}
	{projects}
	{appSettings}
	{labelMap}
	{onEdit}
	{onChangeStatus}
	{onChangePriority}
	{onTogglePin}
	{onDuplicate}
	{onDelete}
	{onAddTask}
/>
