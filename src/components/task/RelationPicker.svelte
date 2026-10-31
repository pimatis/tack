<script lang="ts">
	import { searchTaskCandidates, type ChildTask } from '$lib/repositories/relation.repository';
	import { Input } from '$lib/components/ui/input/index.js';
	import StatusIcon from './StatusIcon.svelte';
	import type { TaskStatus } from '$lib/types/task';

	let {
		excludeId,
		placeholder = 'Search tasks...',
		onPick
	}: {
		excludeId: string;
		placeholder?: string;
		onPick: (task: ChildTask) => void;
	} = $props();

	let query = $state('');
	let results = $state<ChildTask[]>([]);
	let open = $state(false);
	let debounce: ReturnType<typeof setTimeout> | null = null;

	async function search(q: string) {
		try {
			results = await searchTaskCandidates(q, excludeId);
		} catch {
			results = [];
		}
	}

	$effect(() => {
		const q = query;
		if (debounce) clearTimeout(debounce);
		debounce = setTimeout(() => void search(q), 150);
	});
</script>

<div class="relative">
	<Input
		bind:value={query}
		{placeholder}
		class="h-8 w-full rounded-lg border-input bg-transparent px-2.5 text-[12px] text-foreground outline-none placeholder:text-muted-foreground/50 dark:bg-input/30"
		onfocus={() => (open = true)}
		onblur={() => setTimeout(() => (open = false), 150)}
	/>
	{#if open && results.length > 0}
		<div
			class="absolute top-9 right-0 left-0 z-50 max-h-56 overflow-y-auto rounded-lg border border-border bg-popover p-1 shadow-lg"
		>
			{#each results as task (task.id)}
				<button
					type="button"
					class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors hover:bg-muted/50"
					onmousedown={(e) => {
						e.preventDefault();
						onPick(task);
						query = '';
						open = false;
					}}
				>
					<StatusIcon status={task.status as TaskStatus} size={13} />
					<span class="shrink-0 font-mono text-[11px] text-muted-foreground/50">
						{task.prefix ?? 'TSK'}-{task.number}
					</span>
					<span class="min-w-0 flex-1 truncate">{task.title}</span>
				</button>
			{/each}
		</div>
	{/if}
</div>
