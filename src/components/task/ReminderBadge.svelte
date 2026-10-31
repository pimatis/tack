<script lang="ts">
	import { reminderInfo } from '$lib/task/utils';
	import Icon from '$lib/components/Icon.svelte';

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
		<Icon icon="alarm" {size} />
	</span>
{/if}
