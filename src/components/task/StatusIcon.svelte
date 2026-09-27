<script lang="ts">
	import type { TaskStatus } from '$lib/types/task';

	type Props = {
		status: TaskStatus;
		size?: number;
	};

	let { status, size = 14 }: Props = $props();
</script>

{#if status === 'todo'}
	<!-- hollow circle -->
	<svg
		style="width:{size}px;height:{size}px"
		viewBox="0 0 24 24"
		fill="none"
		class="shrink-0 text-muted-foreground/50"
	>
		<circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2" />
	</svg>
{:else if status === 'in_progress'}
	<!-- three-quarter filled circle -->
	<svg
		style="width:{size}px;height:{size}px"
		viewBox="0 0 24 24"
		fill="none"
		class="shrink-0 text-amber-500"
	>
		<circle cx="12" cy="12" r="8" fill="currentColor" opacity="0.2" />
		<path
			d="M12 4a8 8 0 0 1 8 8a8 8 0 0 1-8 8a8 8 0 0 1-8-8a8 8 0 0 1 8-8Z M12 4a8 8 0 0 1 0 16Z"
			fill="currentColor"
		/>
		<circle
			cx="12"
			cy="12"
			r="8"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			opacity="0.5"
		/>
	</svg>
{:else if status === 'done'}
	<!-- green circle with checkmark -->
	<svg
		style="width:{size}px;height:{size}px"
		viewBox="0 0 24 24"
		fill="none"
		class="shrink-0 text-green-500"
	>
		<circle cx="12" cy="12" r="8" fill="currentColor" />
		<path
			fill="none"
			stroke="#0a0a0a"
			stroke-linecap="round"
			stroke-linejoin="round"
			stroke-width="2.5"
			d="m9 12l2 2l4-4"
		/>
	</svg>
{:else if status === 'canceled'}
	<!-- gray circle with x -->
	<svg
		style="width:{size}px;height:{size}px"
		viewBox="0 0 24 24"
		fill="none"
		class="shrink-0 text-muted-foreground/40"
	>
		<circle cx="12" cy="12" r="8" fill="currentColor" />
		<path
			fill="none"
			stroke="#0a0a0a"
			stroke-linecap="round"
			stroke-width="2.5"
			d="m9.5 9.5l5 5m-5 0l5-5"
		/>
	</svg>
{/if}
