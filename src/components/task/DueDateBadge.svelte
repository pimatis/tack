<script lang="ts">
	import { dueDateInfo } from '$lib/task/utils';
	import Icon from '$lib/components/Icon.svelte';

	let { dueDate }: { dueDate?: string | null } = $props();
</script>

{#if dueDateInfo(dueDate)}
	{@const dd = dueDateInfo(dueDate)!}
	<span
		class="flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[11px] font-medium {dd.textColor} {dd.bgColor}"
	>
		<Icon icon="calendar" size={10} />
		{dd.label}
	</span>
{/if}
