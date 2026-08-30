<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';

	let { stats }: { stats: { projects: number; tasks: number; done: number; labels: number } } =
		$props();

	const completionRate = $derived(stats.tasks > 0 ? Math.round((stats.done / stats.tasks) * 100) : 0);
</script>

<div class="grid grid-cols-2 gap-4">
	<Card.Root size="sm">
		<Card.Content class="py-1">
			<p class="text-xs text-muted-foreground">Projects</p>
			<p class="mt-1 text-2xl font-semibold tabular-nums">{stats.projects}</p>
		</Card.Content>
	</Card.Root>
	<Card.Root size="sm">
		<Card.Content class="py-1">
			<p class="text-xs text-muted-foreground">Tasks</p>
			<p class="mt-1 text-2xl font-semibold tabular-nums">{stats.tasks}</p>
		</Card.Content>
	</Card.Root>
	<Card.Root size="sm">
		<Card.Content class="py-1">
			<p class="text-xs text-muted-foreground">Completed</p>
			<p class="mt-1 text-2xl font-semibold tabular-nums">{stats.done}</p>
		</Card.Content>
	</Card.Root>
	<Card.Root size="sm">
		<Card.Content class="py-1">
			<p class="text-xs text-muted-foreground">Completion rate</p>
			<p class="mt-1 text-2xl font-semibold tabular-nums">{completionRate}%</p>
		</Card.Content>
	</Card.Root>
	<Card.Root size="sm">
		<Card.Content class="py-1">
			<p class="text-xs text-muted-foreground">Labels</p>
			<p class="mt-1 text-2xl font-semibold tabular-nums">{stats.labels}</p>
		</Card.Content>
	</Card.Root>
</div>
