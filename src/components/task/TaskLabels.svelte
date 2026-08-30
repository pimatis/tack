<script lang="ts">
	import { labelColorMap, type Label } from '$lib/types/label';

	let {
		labelIds,
		labelMap,
		max = 3
	}: {
		labelIds: string[];
		labelMap: Map<string, Label>;
		max?: number;
	} = $props();
</script>

{#if labelIds.length > 0}
	<div class="flex shrink-0 items-center gap-1">
		{#each labelIds.slice(0, max) as labelId (labelId)}
			{@const label = labelMap.get(labelId)}
			{#if label}
				<span
					class="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[10px] font-medium {labelColorMap[
						label.color
					].badge}"
				>
					<span class="size-1.5 rounded-full {labelColorMap[label.color].dot}"></span>
					{label.name}
				</span>
			{/if}
		{/each}
		{#if labelIds.length > max}
			<span class="text-[10px] text-muted-foreground/50">+{labelIds.length - max}</span>
		{/if}
	</div>
{/if}
