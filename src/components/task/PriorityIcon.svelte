<script lang="ts">
	import Icon from '$lib/components/Icon.svelte';
	import { priorityConfig } from '$lib/task/constants';

	let {
		priority,
		size = 14,
		className = ''
	}: { priority: number; size?: number; className?: string } = $props();
</script>

{#if priority === 1}
	<Icon
		icon="alert-triangle"
		{size}
		class="shrink-0 {priorityConfig[priority].color} {className}"
	/>
{:else}
	<Icon icon="flag" {size} class="shrink-0 {priorityConfig[priority].color} {className}" />
{/if}
