<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import Icon from '$lib/components/Icon.svelte';

	let {
		loading,
		error,
		tasksEmpty,
		filteredEmpty,
		onRefresh,
		onClearFilters
	}: {
		loading: boolean;
		error: string | null;
		tasksEmpty: boolean;
		filteredEmpty: boolean;
		onRefresh: () => void;
		onClearFilters: () => void;
	} = $props();
</script>

{#if loading}
	<div
		class="flex items-center justify-center gap-2 px-4 py-16 text-[13px] text-muted-foreground sm:py-20"
	>
		<Spinner class="size-3.5" />
		<span>Loading tasks...</span>
	</div>
{:else if error}
	<div class="flex flex-col items-center gap-3 px-4 py-16 text-center sm:py-20">
		<div class="flex size-8 items-center justify-center rounded-lg bg-destructive/10">
			<Icon icon="close-circle" size={16} class="text-destructive" />
		</div>
		<p class="text-[13px] text-destructive" role="alert">{error}</p>
		<Button variant="outline" size="sm" onclick={onRefresh}>Try again</Button>
	</div>
{:else if tasksEmpty}
	<div class="flex flex-col items-center justify-center gap-5 px-4 py-20 text-center sm:py-28">
		<div class="flex size-14 items-center justify-center rounded-2xl bg-muted/50">
			<Icon icon="inbox" size={28} class="text-muted-foreground/60" />
		</div>
		<div class="flex flex-col items-center gap-1.5">
			<p class="text-[15px] font-semibold text-foreground">No tasks yet</p>
			<p class="text-[13px] text-muted-foreground">Create your first task to get started</p>
		</div>
		<Button
			variant="default"
			size="sm"
			class="gap-1.5 rounded-lg"
			onclick={() => window.dispatchEvent(new Event('open-task-dialog'))}
		>
			<Icon icon="add" size={14} />
			New task
			<span
				class="ml-1 rounded bg-primary-foreground/20 px-1.5 py-0.5 text-[10px] font-semibold text-primary-foreground"
				>C</span
			>
		</Button>
	</div>
{:else if filteredEmpty}
	<div class="flex flex-col items-center gap-3 px-4 py-16 text-center sm:py-24">
		<div class="flex size-10 items-center justify-center rounded-xl bg-muted">
			<Icon icon="search" size={20} class="text-muted-foreground" />
		</div>
		<div class="text-center">
			<p class="text-[14px] font-medium text-foreground">No matching tasks</p>
			<p class="mt-0.5 text-[13px] text-muted-foreground">Try adjusting your filters.</p>
			<Button
				variant="link"
				class="mt-2 h-auto p-0 text-[12px] font-medium text-foreground/70 transition-colors hover:text-foreground"
				onclick={onClearFilters}>Clear all filters</Button
			>
		</div>
	</div>
{/if}
