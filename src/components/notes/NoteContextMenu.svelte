<script lang="ts">
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import type { NoteInfo } from '$lib/notes/notesState.svelte';

	let {
		note,
		archived = false,
		pinned = false,
		onRename,
		onInfo,
		onTogglePin,
		onArchive,
		onRestore,
		onDelete
	}: {
		note: NoteInfo;
		archived?: boolean;
		pinned?: boolean;
		onRename: (note: NoteInfo) => void;
		onInfo: (note: NoteInfo) => void;
		onTogglePin?: (note: NoteInfo) => void;
		onArchive: (note: NoteInfo) => void;
		onRestore: (note: NoteInfo) => void;
		onDelete: (note: NoteInfo) => void;
	} = $props();
</script>

<ContextMenu.Content>
	{#if onTogglePin && !archived}
		<ContextMenu.Item onclick={() => onTogglePin(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M16.735 2.835a2 2 0 0 0-2.615-.186l-2.913 2.185a9 9 0 0 1-4.127 1.71l-2.177.31c-.73.105-1.265.891-.913 1.662.331.723 1.385 2.629 4.36 5.72l-4.178 4.178a1 1 0 1 0 1.414 1.414l4.178-4.178c3.091 2.975 4.997 4.029 5.72 4.36.77.352 1.557-.183 1.661-.913l.311-2.177a9 9 0 0 1 1.71-4.127L21.35 9.88a2 2 0 0 0-.186-2.615z"
				/></svg
			>
			{#if pinned}
				Unpin
			{:else}
				Pin
			{/if}
		</ContextMenu.Item>
	{/if}
	<ContextMenu.Item onclick={() => onRename(note)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
			/></svg
		>
		Change title
	</ContextMenu.Item>
	{#if archived}
		<ContextMenu.Item onclick={() => onRestore(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M2.614 5.426A1.5 1.5 0 0 1 4 4.5h10a7.5 7.5 0 1 1 0 15H5a1.5 1.5 0 0 1 0-3h9a4.5 4.5 0 1 0 0-9H7.621l.94.94a1.5 1.5 0 0 1-2.122 2.12l-3.5-3.5a1.5 1.5 0 0 1-.325-1.634Z"
				/></svg
			>
			Restore
		</ContextMenu.Item>
	{:else}
		<ContextMenu.Item onclick={() => onArchive(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M7.414 3A2 2 0 0 0 6 3.586L3.586 6a2 2 0 0 0-.543 1h17.914a2 2 0 0 0-.543-1L18 3.586A2 2 0 0 0 16.586 3zM21 9H3v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2zm-9 2a1 1 0 0 1 1 1v3.186l.414-.415a1 1 0 0 1 1.414 1.415l-2.12 2.121a1 1 0 0 1-1.415 0l-2.121-2.121a1 1 0 0 1 1.414-1.415l.414.415V12a1 1 0 0 1 1-1"
				/></svg
			>
			Archive
		</ContextMenu.Item>
	{/if}
	<ContextMenu.Item onclick={() => onInfo(note)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m-.01 8H11a1 1 0 0 0-.117 1.993L11 12v4.99c0 .52.394.95.9 1.004l.11.006h.49a1 1 0 0 0 .596-1.803L13 16.134V11.01c0-.52-.394-.95-.9-1.004zM12 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2"
			/></svg
		>
		Get info
	</ContextMenu.Item>
	<ContextMenu.Separator />
	<ContextMenu.Item variant="destructive" onclick={() => onDelete(note)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
			/></svg
		>
		Delete
	</ContextMenu.Item>
</ContextMenu.Content>
