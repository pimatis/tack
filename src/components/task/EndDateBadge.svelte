<script lang="ts">
	import Icon from '$lib/components/Icon.svelte';

	let { endDate }: { endDate?: string | null } = $props();

	function formatEndDate(dateStr: string): string {
		return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric' }).format(
			new Date(dateStr + 'T00:00:00')
		);
	}
</script>

{#if endDate}
	<span
		class="flex items-center gap-1 rounded-md border border-border/60 px-1.5 py-0.5 text-[11px] font-medium text-muted-foreground/70"
		title={`Ends ${endDate}`}
	>
		<Icon icon="flag" size={10} />
		{formatEndDate(endDate)}
	</span>
{/if}
