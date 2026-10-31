<script lang="ts">
	import type { Task } from '$lib/types/task';
	import { trackedSeconds, formatSeconds } from '$lib/task/time';
	import Icon from '$lib/components/Icon.svelte';

	let { task }: { task: Task } = $props();

	// tick while a timer runs so the displayed total stays live
	let now = $state(Date.now());
	$effect(() => {
		if (!task.timerStartedAt) return;
		const timer = setInterval(() => (now = Date.now()), 30_000);
		return () => clearInterval(timer);
	});

	const total = $derived(trackedSeconds(task, now));
</script>

{#if total > 0 || task.timerStartedAt}
	<span
		class="flex shrink-0 items-center gap-1 text-[11px] tabular-nums {task.timerStartedAt
			? 'text-amber-400'
			: 'text-muted-foreground/50'}"
		title={task.timerStartedAt ? 'Timer running' : 'Tracked time'}
	>
		<Icon icon="stopwatch" size={12} />{formatSeconds(total)}
	</span>
{/if}
