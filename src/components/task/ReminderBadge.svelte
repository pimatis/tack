<script lang="ts">
	import { reminderInfo } from '$lib/task/utils';

	let {
		reminderAt,
		sentAt,
		size = 13
	}: { reminderAt?: string | null; sentAt?: string | null; size?: number } = $props();

	const info = $derived(reminderInfo(reminderAt, sentAt));
</script>

{#if info}
	<span
		class="flex shrink-0 items-center justify-center {info.soon
			? 'text-amber-400'
			: 'text-muted-foreground/50'}"
		role="img"
		aria-label="Reminder {info.label}"
		title="Reminder {info.label}"
	>
		<svg width={size} height={size} viewBox="0 0 24 24" fill="none">
			<path
				fill="currentColor"
				fill-rule="evenodd"
				d="M6.972 3.777a1 1 0 1 0-1.258-1.554 10.038 10.038 0 0 0-2.602 3.19 1 1 0 1 0 1.776.919 8.038 8.038 0 0 1 2.084-2.555m11.314-1.554a1 1 0 1 0-1.258 1.554 8.038 8.038 0 0 1 2.09 2.568 1 1 0 1 0 1.778-.916 10.04 10.04 0 0 0-2.61-3.206M5 10a7 7 0 0 1 14 0v3.764l1.822 3.644A1.1 1.1 0 0 1 19.838 19H4.162a1.1 1.1 0 0 1-.984-1.592L5 13.764zm4 10h6a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2"
			/>
		</svg>
	</span>
{/if}
