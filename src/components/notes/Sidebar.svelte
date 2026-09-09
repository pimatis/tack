<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { notesState, type NoteInfo } from '$lib/notes/notesState.svelte';
	import NoteContextMenu from './NoteContextMenu.svelte';
	import { isTauri } from '$lib/db/client';

	const folderName = $derived(notesState.folder?.split('/').filter(Boolean).pop() ?? '');
	const noteTitle = $derived(notesState.selectedPath?.split('/').pop() ?? '');

	// rename dialog state
	let renameDialogOpen = $state(false);
	let renameTarget = $state<NoteInfo | null>(null);
	let renameDraft = $state('');
	let renameInput = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (renameDialogOpen) requestAnimationFrame(() => renameInput?.focus());
	});

	function openRenameDialog(note: NoteInfo) {
		renameTarget = note;
		renameDraft = note.name.replace(/\.md$/, '');
		renameDialogOpen = true;
	}

	function submitRename(event: SubmitEvent) {
		event.preventDefault();
		if (!renameTarget || !renameDraft.trim()) return;
		void notesState.renameNote(renameTarget.path, renameDraft);
		renameDialogOpen = false;
	}

	// collapsed state of the archived notes list
	let archivedOpen = $state(true);
</script>

{#if !isTauri()}
	<div class="px-4 py-6 text-center text-[12px] text-muted-foreground">
		Notes are only available in the desktop app.
	</div>
{:else if !notesState.folder}
	<!-- no folder yet: pick one before notes can exist -->
	<div class="flex flex-col items-center gap-3 px-4 py-8 text-center">
		<svg class="text-muted-foreground/50" width="24" height="24" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M4 4a2 2 0 0 1 2-2h4a2 2 0 0 1 1.7 1l.55.83a.5.5 0 0 0 .42.17H18a2 2 0 0 1 2 2v1H7.66a3 3 0 0 0-2.82 2L2.9 14.4A1 1 0 0 1 2 14V4m0 16a2 2 0 0 0 2 2h13.34a3 3 0 0 0 2.82-2l2.4-7.2A1 1 0 0 0 23.6 12a2 2 0 0 0-1.9-2H7.66a1 1 0 0 0-.94.67L3 20z"
			/></svg
		>
		<p class="text-[12px] leading-relaxed text-muted-foreground">
			Pick a folder to store your notes
		</p>
		<Button size="sm" variant="outline" onclick={() => void notesState.pickFolder()}>
			Choose folder
		</Button>
	</div>
{:else}
	{#if !notesState.showArchived}
		<!-- open folder + new note -->
		<div class="flex items-center gap-1 px-3 pt-1 pb-1">
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<button
							{...props}
							type="button"
							class="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-1 py-1 text-left text-[12px] font-medium text-muted-foreground transition-colors hover:text-sidebar-foreground"
							onclick={() => void notesState.pickFolder()}
						>
							<svg class="shrink-0" width="13" height="13" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M21.328 10a.5.5 0 0 1 .496.563l-.017.08-2.89 9.644a1 1 0 0 1-.84.706L17.96 21H4a1.99 1.99 0 0 1-1.099-.328.494.494 0 0 1-.026-.234l.017-.082 2.894-9.643a1 1 0 0 1 .839-.706L6.744 10zM9.52 3a2 2 0 0 1 1.443.614l.12.137L12.48 5.5H19a2 2 0 0 1 1.995 1.85L21 7.5V8H6.744A3 3 0 0 0 3.93 9.96l-.06.178L2 16.37V5a2 2 0 0 1 1.85-1.995L4 3z"
								/></svg
							>
							<span class="truncate">{folderName}</span>
						</button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="right">Change folder</Tooltip.Content>
			</Tooltip.Root>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
							aria-label="Sort notes"
						>
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M16.94 3.111a1.5 1.5 0 0 1 2.007-.103l.114.103 2.828 2.829a1.5 1.5 0 0 1-2.007 2.224l-.114-.103-.268-.268V19a1.5 1.5 0 0 1-2.993.144L16.5 19V7.793l-.268.268a1.5 1.5 0 0 1-2.224-2.008l.103-.113 2.828-2.829ZM13 17.5a1.5 1.5 0 0 1 .144 2.993L13 20.5H4a1.5 1.5 0 0 1-.144-2.993L4 17.5zm0-7a1.5 1.5 0 0 1 0 3H4a1.5 1.5 0 0 1 0-3zm-2-7a1.5 1.5 0 0 1 0 3H4a1.5 1.5 0 1 1 0-3z"
								/></svg
							>
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="end" class="w-40">
					<DropdownMenu.RadioGroup
						value={notesState.sortBy}
						onValueChange={(v) => notesState.setSort(v as 'modified' | 'name')}
					>
						<DropdownMenu.RadioItem value="modified">Last modified</DropdownMenu.RadioItem>
						<DropdownMenu.RadioItem value="name">Title</DropdownMenu.RadioItem>
					</DropdownMenu.RadioGroup>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
							onclick={() => void notesState.createNote()}
							aria-label="New note"
						>
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
								/></svg
							>
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content side="right">New note</Tooltip.Content>
			</Tooltip.Root>
		</div>
	{/if}

	{#if notesState.error}
		<div class="px-3 py-2 text-[12px] text-red-400/90">{notesState.error}</div>
	{:else if notesState.loading}
		<div class="px-3 py-2 text-[12px] text-muted-foreground/60">Loading…</div>
	{:else if notesState.showArchived}
		<!-- archived notes: title goes back to notes, chevron collapses the list -->
		<div
			class="flex items-center gap-1 rounded-md px-3 pt-1 pb-1 text-[13px] font-medium text-muted-foreground transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
		>
			<button
				type="button"
				class="flex shrink-0 items-center"
				onclick={() => (archivedOpen = !archivedOpen)}
				aria-label="Toggle archived notes"
			>
				<svg
					class="transition-transform duration-150 {archivedOpen ? 'rotate-90' : ''}"
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					><path
						fill="currentColor"
						d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
					/></svg
				>
			</button>
			<button
				type="button"
				class="min-w-0 flex-1 text-left"
				onclick={() => (notesState.showArchived = false)}
			>
				<span>Archived Notes</span>
			</button>
			<span class="pr-1 text-[11px] text-muted-foreground/60 tabular-nums"
				>{notesState.archived.length}</span
			>
		</div>
		{#if archivedOpen}
			{#if notesState.archived.length === 0}
				<div class="px-3 py-3 text-[12px] text-muted-foreground/60">Nothing archived yet</div>
			{:else}
				<div class="mt-0.5 grid gap-px px-1.5">
					{#each notesState.archived as note (note.path)}
						<ContextMenu.Root>
							<ContextMenu.Trigger
								class="flex w-full min-w-0 items-center rounded-md transition-colors hover:bg-sidebar-accent"
							>
								<button
									type="button"
									class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {noteTitle ===
									note.name
										? 'bg-sidebar-accent/70 text-sidebar-foreground'
										: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
									onclick={() => void notesState.openNote(note.path)}
								>
									<svg
										class="shrink-0 text-muted-foreground"
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5zM8 12h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2m0 4h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2"
										/></svg
									>
									<span class="truncate">{note.name.replace(/\.md$/, '')}</span>
								</button>
							</ContextMenu.Trigger>
							<NoteContextMenu
								{note}
								archived
								onRename={openRenameDialog}
								onArchive={(n) => void notesState.archiveNote(n.path)}
								onRestore={(n) => void notesState.restoreNote(n.path)}
								onDelete={(n) => void notesState.deleteNote(n.path)}
							/>
						</ContextMenu.Root>
					{/each}
				</div>
			{/if}
		{/if}
	{:else if notesState.notes.length === 0}
		<div class="px-3 py-3 text-[12px] text-muted-foreground/60">No notes yet</div>
	{:else}
		<!-- notes list: pinned first, then by the chosen sort order -->
		<div class="mt-0.5 grid gap-px px-1.5">
			{#each notesState.sortedNotes as note (note.path)}
				<ContextMenu.Root>
					<ContextMenu.Trigger
						class="flex w-full min-w-0 items-center rounded-md transition-colors hover:bg-sidebar-accent"
					>
						<button
							type="button"
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {noteTitle ===
							note.name
								? 'bg-sidebar-accent/70 text-sidebar-foreground'
								: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
							onclick={() => void notesState.openNote(note.path)}
						>
							<svg
								class="shrink-0 text-muted-foreground"
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								><path
									fill="currentColor"
									d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5zM8 12h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2m0 4h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2"
								/></svg
							>
							<span class="truncate">{note.name.replace(/\.md$/, '')}</span>
							{#if notesState.pinned.includes(note.name)}
								<svg
									class="ml-auto shrink-0 text-muted-foreground/60"
									width="12"
									height="12"
									viewBox="0 0 24 24"
									fill="none"
									><path
										fill="currentColor"
										d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
									/></svg
								>
							{/if}
						</button>
					</ContextMenu.Trigger>
					<NoteContextMenu
						{note}
						pinned={notesState.pinned.includes(note.name)}
						onRename={openRenameDialog}
						onTogglePin={(n) => notesState.togglePin(n.name)}
						onArchive={(n) => void notesState.archiveNote(n.path)}
						onRestore={(n) => void notesState.restoreNote(n.path)}
						onDelete={(n) => void notesState.deleteNote(n.path)}
					/>
				</ContextMenu.Root>
			{/each}
		</div>
	{/if}
{/if}

<Dialog.Root bind:open={renameDialogOpen}>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">Change note title</Dialog.Title>
		<form onsubmit={submitRename} class="flex flex-col">
			<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
				<span class="text-[13px] font-medium text-foreground">Change note title</span>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:text-foreground"
						>
							<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
								><path
									fill="currentColor"
									d="m12 13.6 3.7 3.7a1 1 0 0 0 1.4-1.4L13.4 12l3.7-3.7a1 1 0 0 0-1.4-1.4L12 10.6 8.3 6.9a1 1 0 1 0-1.4 1.4l3.7 3.7-3.7 3.7a1 1 0 1 0 1.4 1.4z"
								/></svg
							>
						</Button>
					{/snippet}
				</Dialog.Close>
			</div>
			<div class="px-4 pb-4 sm:px-5">
				<Input bind:ref={renameInput} bind:value={renameDraft} placeholder="Note title" />
			</div>
			<div class="flex justify-end gap-2 px-4 pb-4 sm:px-5">
				<Button type="button" variant="outline" size="sm" onclick={() => (renameDialogOpen = false)}
					>Cancel</Button
				>
				<Button type="submit" size="sm">Rename</Button>
			</div>
		</form>
	</Dialog.Content>
</Dialog.Root>
