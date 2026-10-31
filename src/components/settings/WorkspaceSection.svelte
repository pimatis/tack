<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';

	let { stats }: { stats: { projects: number; tasks: number; done: number; labels: number } } =
		$props();

	const completionRate = $derived(
		stats.tasks > 0 ? Math.round((stats.done / stats.tasks) * 100) : 0
	);

	const items = $derived([
		{ label: 'Projects', value: stats.projects },
		{ label: 'Tasks', value: stats.tasks },
		{ label: 'Completed', value: stats.done },
		{ label: 'Completion rate', value: `${completionRate}%` },
		{ label: 'Labels', value: stats.labels }
	]);
</script>

<div class="grid grid-cols-1 gap-3 min-[480px]:grid-cols-2 sm:gap-4">
	{#each items as item (item.label)}
		<Card.Root size="sm">
			<Card.Header>
				<Card.Description class="text-xs">{item.label}</Card.Description>
				<Card.Title class="text-2xl font-semibold tabular-nums">{item.value}</Card.Title>
			</Card.Header>
		</Card.Root>
	{/each}
</div>
