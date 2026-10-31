<script lang="ts">
	import { recurrenceLabel } from '$lib/task/recurrence';
	import Icon from '$lib/components/Icon.svelte';

	let { rule, size = 13 }: { rule?: string | null; size?: number } = $props();

	const label = $derived(recurrenceLabel(rule));
</script>

{#if label}
	<span
		class="flex shrink-0 items-center justify-center text-muted-foreground/50"
		role="img"
		aria-label={label}
		title={label}
	>
		<Icon icon="repeat" {size} />
	</span>
{/if}
