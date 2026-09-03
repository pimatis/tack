<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';

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
			<svg class="text-destructive" width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m-.01 8H11a1 1 0 0 0-.117 1.993L11 12v4.99c0 .52.394.95.9 1.004l.11.006h.49a1 1 0 0 0 .596-1.803L13 16.134V11.01c0-.52-.394-.95-.9-1.004zM12 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2"
				/></svg
			>
		</div>
		<p class="text-[13px] text-destructive" role="alert">{error}</p>
		<Button variant="outline" size="sm" onclick={onRefresh}>Try again</Button>
	</div>
{:else if tasksEmpty}
	<div class="flex flex-col items-center justify-center gap-5 px-4 py-20 text-center sm:py-28">
		<div class="flex size-14 items-center justify-center rounded-2xl bg-muted/50">
			<svg class="text-muted-foreground/60" width="28" height="28" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M5.83 5.106A2 2 0 0 1 7.617 4h8.764a2 2 0 0 1 1.789 1.106l3.512 7.025a3 3 0 0 1 .318 1.34V19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-5.528a3 3 0 0 1 .317-1.341zM16.381 6H7.618L4.12 13H7.5A1.5 1.5 0 0 1 9 14.5v1a.5.5 0 0 0 .5.5h5a.5.5 0 0 0 .5-.5v-1a1.5 1.5 0 0 1 1.5-1.5h3.38z"
				/></svg
			>
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
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
				/></svg
			>
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
			<svg class="text-muted-foreground" width="20" height="20" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
				/></svg
			>
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
