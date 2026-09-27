<script lang="ts">
	import { priorityConfig } from '$lib/task/constants';

	let {
		priority,
		size = 14,
		className = ''
	}: { priority: number; size?: number; className?: string } = $props();
</script>

{#if priority === 1}
	<svg
		class="shrink-0 {priorityConfig[priority].color} {className}"
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
		><path
			fill="currentColor"
			d="M10.7 3.148a1.5 1.5 0 0 1 2.6 0l8.633 14.954a1.5 1.5 0 0 1-1.299 2.25H3.366a1.5 1.5 0 0 1-1.299-2.25zM12 15.001a1 1 0 1 0 0 2a1 1 0 0 0 0-2m0-7a1 1 0 0 0-1 1v4a1 1 0 0 0 2 0v-4a1 1 0 0 0-1-1"
		/></svg
	>
{:else}
	<svg
		class="shrink-0 {priorityConfig[priority].color} {className}"
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
	>
		<rect
			x="3"
			y="14"
			width="3.5"
			height="7"
			rx="1"
			fill="currentColor"
			opacity={priority > 0 ? 1 : 0.25}
		/>
		<rect
			x="10.25"
			y="9"
			width="3.5"
			height="12"
			rx="1"
			fill="currentColor"
			opacity={priority > 0 && priority < 4 ? 1 : 0.25}
		/>
		<rect
			x="17.5"
			y="4"
			width="3.5"
			height="17"
			rx="1"
			fill="currentColor"
			opacity={priority > 0 && priority < 3 ? 1 : 0.25}
		/>
	</svg>
{/if}
