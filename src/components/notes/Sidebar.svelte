<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { sortableItem, dropZone, reorderArray, type DragDropState } from '$lib/dnd';
	import { notesState, type NoteInfo } from '$lib/notes/notesState.svelte';
	import NoteContextMenu from './NoteContextMenu.svelte';
	import { isTauri } from '$lib/db/client';

	const folderName = $derived(notesState.folder?.split('/').filter(Boolean).pop() ?? '');
	const noteTitle = $derived(notesState.selectedPath?.split('/').pop() ?? '');

	// what a drag carries through the dnd layer
	type NoteDrag = { kind: 'note'; name: string; path: string } | { kind: 'folder'; rel: string };

	// rename dialog state; works for notes and folders
	let renameDialogOpen = $state(false);
	let renameTargetNote = $state<NoteInfo | null>(null);
	let renameTargetFolder = $state<string | null>(null);
	let renameDraft = $state('');
	let renameInput = $state<HTMLInputElement | null>(null);

	$effect(() => {
		if (renameDialogOpen) requestAnimationFrame(() => renameInput?.focus());
	});

	function openRenameDialog(note: NoteInfo) {
		renameTargetNote = note;
		renameTargetFolder = null;
		renameDraft = note.name.replace(/\.md$/, '');
		renameDialogOpen = true;
	}

	function openFolderRenameDialog(rel: string) {
		renameTargetNote = null;
		renameTargetFolder = rel;
		renameDraft = rel.split('/').pop() ?? '';
		renameDialogOpen = true;
	}

	function submitRename(event: SubmitEvent) {
		event.preventDefault();
		if (renameTargetNote && renameDraft.trim()) {
			void notesState.renameNote(renameTargetNote.path, renameDraft);
		} else if (renameTargetFolder && renameDraft.trim()) {
			void notesState.renameFolder(renameTargetFolder, renameDraft);
		}
		renameDialogOpen = false;
	}

	// container id for a folder level; '' means the notes root
	function containerId(rel: string | null): string {
		return rel ? `notes:${rel}` : 'notes:root';
	}

	function relFromContainer(id: string | undefined): string | null {
		if (!id?.startsWith('notes:')) return null;
		const rel = id.slice('notes:'.length);
		return rel === 'root' || rel === '' ? null : rel;
	}

	// immediate subfolders of a folder, in stable order
	function childFolders(rel: string | null): string[] {
		return notesState.folders.filter((f) => {
			const parent = f.includes('/') ? f.slice(0, f.lastIndexOf('/')) : null;
			return (parent ?? null) === rel;
		});
	}

	// drop onto a note row: reorder inside the folder, or move between folders
	function handleNoteDrop(
		state: DragDropState<NoteDrag>,
		target: NoteInfo,
		targetRel: string | null
	) {
		const dragged = state.draggedItem;
		if (!dragged) return;
		if (dragged.kind === 'folder') {
			void notesState.moveFolder(dragged.rel, targetRel);
			return;
		}
		if (dragged.path === target.path) return;
		const sourceRel = relFromContainer(state.sourceContainer);
		const container = containerId(targetRel);
		if (sourceRel === targetRel && state.dropPosition) {
			const list =
				targetRel === null ? notesState.sortedNotes : notesState.sortedFolderNotes(targetRel);
			const draggedNote = list.find((n) => n.path === dragged.path);
			if (!draggedNote) return;
			notesState.applyOrder(
				container,
				reorderArray(list, draggedNote, target, state.dropPosition!)
			);
		} else {
			void notesState.moveNote(dragged.path, targetRel);
		}
	}

	// drop onto a folder row: anything goes into that folder
	function handleFolderDrop(state: DragDropState<NoteDrag>, rel: string) {
		const dragged = state.draggedItem;
		if (!dragged) return;
		if (dragged.kind === 'note') void notesState.moveNote(dragged.path, rel);
		else void notesState.moveFolder(dragged.rel, rel);
	}

	// drop onto the root area: back out of folders
	function handleRootDrop(state: DragDropState<NoteDrag>) {
		const dragged = state.draggedItem;
		if (!dragged) return;
		if (dragged.kind === 'note') void notesState.moveNote(dragged.path, null);
		else void notesState.moveFolder(dragged.rel, null);
	}

	function openCreateDialog(type: 'note' | 'folder', parent: string | null) {
		window.dispatchEvent(new CustomEvent('open-note-create-dialog', { detail: { type, parent } }));
	}
</script>

{#snippet noteRow(note: NoteInfo, rel: string | null)}
	<ContextMenu.Root>
		<ContextMenu.Trigger class="contents">
			<div
				role="listitem"
				use:sortableItem={{
					dragData: { kind: 'note' as const, name: note.name, path: note.path },
					container: containerId(rel),
					onDrop: (s: DragDropState<NoteDrag>) => handleNoteDrop(s, note, rel)
				}}
			>
				<div
					role="button"
					tabindex="0"
					class="flex w-full cursor-grab items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors {noteTitle ===
					note.name
						? 'bg-sidebar-accent/70 text-sidebar-foreground'
						: 'text-sidebar-foreground/80 hover:bg-sidebar-accent/50 hover:text-sidebar-foreground'}"
					onclick={() => void notesState.openNote(note.path)}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') {
							e.preventDefault();
							void notesState.openNote(note.path);
						}
					}}
				>
					<svg
						class="shrink-0 text-muted-foreground"
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d="M18 2a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2zm-6 11H9a1 1 0 1 0 0 2h3a1 1 0 1 0 0-2m3-5H9a1 1 0 0 0-.117 1.993L9 10h6a1 1 0 0 0 .117-1.993z"
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
				</div>
			</div>
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
{/snippet}

{#snippet folderNode(rel: string)}
	<div
		role="listitem"
		use:sortableItem={{
			dragData: { kind: 'folder' as const, rel },
			container: containerId(rel),
			onDrop: (s: DragDropState<NoteDrag>) => handleFolderDrop(s, rel)
		}}
	>
		<ContextMenu.Root>
			<ContextMenu.Trigger class="contents">
				<div
					role="button"
					tabindex="0"
					class="flex w-full cursor-grab items-center gap-1.5 rounded-md py-1.5 pr-2 text-left text-[13px] text-sidebar-foreground/80 transition-colors hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
					onclick={() => notesState.toggleFolder(rel)}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') {
							e.preventDefault();
							notesState.toggleFolder(rel);
						}
					}}
				>
					<svg
						class="shrink-0 text-muted-foreground transition-transform duration-150 {notesState.isExpanded(
							rel
						)
							? 'rotate-90'
							: ''}"
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
						/></svg
					>
					<svg
						class="shrink-0 text-muted-foreground"
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d="M21.328 10a.5.5 0 0 1 .496.563l-.017.08-2.89 9.644a1 1 0 0 1-.84.706L17.96 21H4a1.99 1.99 0 0 1-1.099-.328.494.494 0 0 1-.026-.234l.017-.082 2.894-9.643a1 1 0 0 1 .839-.706L6.744 10zM9.52 3a2 2 0 0 1 1.443.614l.12.137L12.48 5.5H19a2 2 0 0 1 1.995 1.85L21 7.5V8H6.744A3 3 0 0 0 3.93 9.96l-.06.178L2 16.37V5a2 2 0 0 1 1.85-1.995L4 3z"
						/></svg
					>
					<span class="truncate">{rel.split('/').pop()}</span>
					{#if !notesState.isExpanded(rel)}
						<span class="ml-auto text-[11px] text-muted-foreground/50 tabular-nums">
							{(notesState.folderNotes[rel] ?? []).length +
								childFolders(rel).reduce(
									(acc, f) => acc + (notesState.folderNotes[f] ?? []).length,
									0
								)}
						</span>
					{/if}
				</div>
			</ContextMenu.Trigger>
			<ContextMenu.Content class="w-44">
				<ContextMenu.Item onclick={() => openCreateDialog('note', rel)}>
					<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5z"
						/></svg
					>
					New note
				</ContextMenu.Item>
				<ContextMenu.Item onclick={() => openCreateDialog('folder', rel)}>
					<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M21.328 10a.5.5 0 0 1 .496.563l-.017.08-2.89 9.644a1 1 0 0 1-.84.706L17.96 21H4a1.99 1.99 0 0 1-1.099-.328.494.494 0 0 1-.026-.234l.017-.082 2.894-9.643a1 1 0 0 1 .839-.706L6.744 10zM9.52 3a2 2 0 0 1 1.443.614l.12.137L12.48 5.5H19a2 2 0 0 1 1.995 1.85L21 7.5V8H6.744A3 3 0 0 0 3.93 9.96l-.06.178L2 16.37V5a2 2 0 0 1 1.85-1.995L4 3z"
						/></svg
					>
					New subfolder
				</ContextMenu.Item>
				<ContextMenu.Item onclick={() => openFolderRenameDialog(rel)}>
					<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
						/></svg
					>
					Rename
				</ContextMenu.Item>
				<ContextMenu.Separator />
				<ContextMenu.Item variant="destructive" onclick={() => void notesState.deleteFolder(rel)}>
					<svg class="text-muted-foreground" width="16" height="16" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
						/></svg
					>
					Delete folder
				</ContextMenu.Item>
			</ContextMenu.Content>
		</ContextMenu.Root>
	</div>
	{#if notesState.isExpanded(rel)}
		<!-- children indent with a subtle tree guide line, obsidian-style -->
		<div
			class="ml-[13px] grid gap-px border-l border-sidebar-border/50 pl-1.5"
			use:dropZone={{
				container: containerId(rel),
				direction: 'vertical',
				onDrop: (s: DragDropState<NoteDrag>) => {
					const dragged = s.draggedItem;
					if (!dragged) return;
					if (dragged.kind === 'note') void notesState.moveNote(dragged.path, rel);
					else void notesState.moveFolder(dragged.rel, rel);
				}
			}}
		>
			{#each notesState.sortedFolderNotes(rel) as note (note.path)}
				{@render noteRow(note, rel)}
			{/each}
			{#each childFolders(rel) as child (child)}
				{@render folderNode(child)}
			{/each}
		</div>
	{/if}
{/snippet}

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
	<!-- open folder + sort -->
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
					onValueChange={(v) => notesState.setSort(v as 'modified' | 'name' | 'manual')}
				>
					<DropdownMenu.RadioItem value="modified">Last modified</DropdownMenu.RadioItem>
					<DropdownMenu.RadioItem value="name">Title</DropdownMenu.RadioItem>
					<DropdownMenu.RadioItem value="manual">Manual order</DropdownMenu.RadioItem>
				</DropdownMenu.RadioGroup>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</div>

	{#if notesState.error}
		<div class="px-3 py-2 text-[12px] text-red-400/90">{notesState.error}</div>
	{:else if notesState.loading}
		<div class="px-3 py-2 text-[12px] text-muted-foreground/60">Loading…</div>
	{:else}
		<!-- tree: root notes first, then folders with their notes -->
		<div
			class="mt-0.5 grid gap-px px-1.5"
			use:dropZone={{
				container: 'notes:root',
				direction: 'vertical',
				onDrop: handleRootDrop
			}}
		>
			{#if notesState.notes.length === 0 && notesState.folders.length === 0}
				<div class="px-3 py-3 text-[12px] text-muted-foreground/60">No notes yet</div>
			{/if}
			{#each notesState.sortedNotes as note (note.path)}
				{@render noteRow(note, null)}
			{/each}
			{#each childFolders(null) as rel (rel)}
				{@render folderNode(rel)}
			{/each}
		</div>
	{/if}
{/if}

<Dialog.Root bind:open={renameDialogOpen}>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">
			{renameTargetFolder ? 'Rename folder' : 'Change note title'}
		</Dialog.Title>
		<form onsubmit={submitRename} class="flex flex-col">
			<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
				<span class="text-[13px] font-medium text-foreground">
					{renameTargetFolder ? 'Rename folder' : 'Change note title'}
				</span>
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
									d="M18.3 5.71a1 1 0 0 0-1.42 0L12 10.59l-4.88-4.88a1 1 0 1 0-1.42 1.42L10.59 12l-4.88 4.88a1 1 0 1 0 1.41 1.42L12 13.41l4.88 4.88a1 1 0 0 0 1.42-1.42L13.41 12l4.88-4.88a1 1 0 0 0 0-1.41Z"
								/></svg
							>
						</Button>
					{/snippet}
				</Dialog.Close>
			</div>
			<div class="px-4 pb-4 sm:px-5">
				<Input
					bind:ref={renameInput}
					bind:value={renameDraft}
					placeholder={renameTargetFolder ? 'Folder name' : 'Note title'}
				/>
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
