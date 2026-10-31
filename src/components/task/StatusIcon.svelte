<script lang="ts">
	import Icon from '$lib/components/Icon.svelte';
	import type { TaskStatus } from '$lib/types/task';

	type Props = {
		status: TaskStatus;
		size?: number;
	};

	let { status, size = 14 }: Props = $props();
</script>

{#if status === 'backlog'}
	<Icon icon="inbox" {size} class="shrink-0 text-muted-foreground/40" />
{:else if status === 'todo'}
	<Icon icon="circle-outline" {size} class="shrink-0 text-muted-foreground/50" />
{:else if status === 'in_progress'}
	<Icon icon="pie-chart" {size} class="shrink-0 text-amber-500" />
{:else if status === 'done'}
	<Icon icon="check-circle" {size} class="shrink-0 text-green-500" />
{:else if status === 'canceled'}
	<Icon icon="close-circle" {size} class="shrink-0 text-muted-foreground/40" />
{/if}
