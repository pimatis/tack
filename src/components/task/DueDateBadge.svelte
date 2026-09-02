<script lang="ts">
	import { dueDateInfo } from '$lib/task/utils';

	let { dueDate }: { dueDate?: string | null } = $props();
</script>

{#if dueDateInfo(dueDate)}
	{@const dd = dueDateInfo(dueDate)!}
	<span
		class="flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[11px] font-medium {dd.textColor} {dd.bgColor}"
	>
		<svg width="10" height="10" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M16 3a1 1 0 0 1 1 1v1h2a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h2V4a1 1 0 0 1 2 0v1h6V4a1 1 0 0 1 1-3M8.01 16H8a1 1 0 0 0-.117 1.993L8.01 18a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m-8-4H8a1 1 0 0 0-.117 1.993L8.01 14a1 1 0 1 0 0-2m4 0H12a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2m4 0H16a1 1 0 0 0-.117 1.993l.127.007a1 1 0 1 0 0-2M19 7H5v2h14z"
			/></svg
		>
		{dd.label}
	</span>
{/if}
