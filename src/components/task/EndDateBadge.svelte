<script lang="ts">
	let { endDate }: { endDate?: string | null } = $props();

	function formatEndDate(dateStr: string): string {
		return new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' }).format(
			new Date(dateStr + 'T00:00:00')
		);
	}
</script>

{#if endDate}
	<span
		class="flex items-center gap-1 rounded-md border border-border/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground/70"
		title={`Ends ${endDate}`}
	>
		<svg width="10" height="10" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M6 3a2 2 0 0 0-2 2v16a1 1 0 1 0 2 0v-5h13.804a1.1 1.1 0 0 0 .89-1.747L17.236 9.5l3.456-4.753A1.1 1.1 0 0 0 19.803 3z"
			/></svg
		>
		{formatEndDate(endDate)}
	</span>
{/if}
