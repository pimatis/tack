<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';

	let {
		selectedCount,
		isAllSelected,
		onToggleSelectAll,
		onBulkTag,
		onBulkMove,
		onBulkArchive,
		onBulkDelete,
		onClearSelection
	}: {
		selectedCount: number;
		isAllSelected: boolean;
		onToggleSelectAll: () => void;
		onBulkTag: (tag: string, add: boolean) => void;
		onBulkMove: (folderRel: string | null) => void;
		onBulkArchive: () => void;
		onBulkDelete: () => void;
		onClearSelection: () => void;
	} = $props();

	// click semantics per tag: every selected note not carrying it gets it;
	// when all of them already have it the click removes it from all
	function tagState(tag: string): 'all' | 'some' | 'none' {
		let count = 0;
		for (const note of notesState.selectedNotes) {
			if (notesState.tagsOfNote(note.path).includes(tag)) count++;
		}
		return count === 0 ? 'none' : count === notesState.selectedNotes.length ? 'all' : 'some';
	}
</script>

<div class="flex flex-wrap items-center gap-2 px-3 pt-1 pb-1">
	<Checkbox checked={isAllSelected} onCheckedChange={() => onToggleSelectAll()} />
	<span class="text-[12px] font-medium text-foreground">
		{selectedCount}
		{selectedCount === 1 ? 'note' : 'notes'}
		selected
	</span>

	<div class="flex-1"></div>

	<div class="flex flex-wrap items-center gap-1.5">
		<!-- bulk tags -->
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
					>
						<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M10.537 2.164a3 3 0 0 1 2.244.727l.15.14 7.822 7.823a3 3 0 0 1 .135 4.098l-.135.144-5.657 5.657a3 3 0 0 1-4.098.135l-.144-.135L3.03 12.93a3 3 0 0 1-.878-2.188l.011-.205.472-5.185a3 3 0 0 1 2.537-2.695l.179-.021zM8.024 8.025a2 2 0 1 0 2.829 2.829 2 2 0 0 0-2.829-2.829"
							/></svg
						>
						<span>Tags</span>
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-52 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
					Toggle tag for all
				</div>
				{#each notesState.allTags as tag (tag)}
					{@const state = tagState(tag)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onBulkTag(tag, state !== 'all')}
					>
						<span class="text-[12px] text-muted-foreground/70">#{tag}</span>
						{#if state !== 'none'}
							<svg
								class="ml-auto shrink-0 {state === 'some' ? 'opacity-40' : ''}"
								width="13"
								height="13"
								viewBox="0 0 24 24"
								fill="none"
								><path fill="currentColor" d="M9 16.2 4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4z" /></svg
							>
						{/if}
					</Button>
				{/each}
				{#if notesState.allTags.length === 0}
					<div class="px-2 py-1.5 text-[12px] text-muted-foreground">No tags yet</div>
				{/if}
			</Popover.Content>
		</Popover.Root>

		<!-- bulk move to folder -->
		<Popover.Root>
			<Popover.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="outline"
						size="sm"
						class="flex h-8 items-center gap-1.5 rounded-lg border border-input px-2.5 text-[12px] font-medium text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
					>
						<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M21.328 10a.5.5 0 0 1 .496.563l-.017.08-2.89 9.644a1 1 0 0 1-.84.706L17.96 21H4a1.99 1.99 0 0 1-1.099-.328.494.494 0 0 1-.026-.234l.017-.082 2.894-9.643a1 1 0 0 1 .839-.706L6.744 10zM9.52 3a2 2 0 0 1 1.443.614l.12.137L12.48 5.5H19a2 2 0 0 1 1.995 1.85L21 7.5V8H6.744A3 3 0 0 0 3.93 9.96l-.06.178L2 16.37V5a2 2 0 0 1 1.85-1.995L4 3z"
							/></svg
						>
						<span>Move</span>
					</Button>
				{/snippet}
			</Popover.Trigger>
			<Popover.Content class="w-52 p-1.5" align="end">
				<div class="px-2 py-1.5 text-[11px] font-medium text-muted-foreground">Move to folder</div>
				<Button
					variant="ghost"
					class="flex h-auto w-full items-center justify-start rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
					onclick={() => onBulkMove(null)}
				>
					Notes root
				</Button>
				{#each notesState.folders as rel (rel)}
					<Button
						variant="ghost"
						class="flex h-auto w-full items-center justify-start rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						onclick={() => onBulkMove(rel)}
					>
						<span class="truncate">{rel}</span>
					</Button>
				{/each}
				{#if notesState.folders.length === 0}
					<div class="px-2 py-1.5 text-[12px] text-muted-foreground">No folders yet</div>
				{/if}
			</Popover.Content>
		</Popover.Root>

		<!-- bulk archive -->
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-foreground"
						aria-label="Archive selected"
						onclick={() => onBulkArchive()}
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M21 19c0 1.105-.895 2-2 2H5a2 2 0 0 1-2-2V9h18zM12 11a1 1 0 0 0-1 1v3.186l-.586-.415a1 1 0 0 0-1.414 1.414l2.121 2.122a1 1 0 0 0 1.414 0l2.121-2.122a1 1 0 0 0-1.414-1.414l-.586.415V12a1 1 0 0 0-1-1m4.586-8c.53 0 1.039.211 1.414.586l2.414 2.414c.276.276.461.624.541 1H3.045c.08-.376.265-.724.541-1L6 3.586A2 2 0 0 1 7.414 3z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Archive</Tooltip.Content>
		</Tooltip.Root>

		<!-- bulk delete -->
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-destructive"
						aria-label="Delete selected"
						onclick={() => onBulkDelete()}
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Delete</Tooltip.Content>
		</Tooltip.Root>

		<!-- clear selection -->
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						class="text-muted-foreground hover:text-foreground"
						aria-label="Clear selection"
						onclick={() => onClearSelection()}
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
							/></svg
						>
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="bottom">Clear selection</Tooltip.Content>
		</Tooltip.Root>
	</div>
</div>
