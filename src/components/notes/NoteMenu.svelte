<script lang="ts">
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import type { NoteInfo } from '$lib/notes/notesState.svelte';

	let {
		note,
		archived = false,
		pinned = false,
		selected = false,
		onRename,
		onInfo,
		onTogglePin,
		onArchive,
		onRestore,
		onDelete,
		onTags,
		onHistory,
		onExport,
		onConvertToTask,
		onToggleSelect
	}: {
		note: NoteInfo;
		archived?: boolean;
		pinned?: boolean;
		selected?: boolean;
		onRename: (note: NoteInfo) => void;
		onInfo: (note: NoteInfo) => void;
		onTogglePin?: (note: NoteInfo) => void;
		onArchive: (note: NoteInfo) => void;
		onRestore: (note: NoteInfo) => void;
		onDelete: (note: NoteInfo) => void;
		onTags: (note: NoteInfo) => void;
		onHistory: (note: NoteInfo) => void;
		onExport: (note: NoteInfo, format: 'md' | 'html') => void;
		onConvertToTask: (note: NoteInfo) => void;
		onToggleSelect?: (note: NoteInfo) => void;
	} = $props();
</script>

<DropdownMenu.Content class="w-52">
	{#if onToggleSelect}
		<DropdownMenu.Item onclick={() => onToggleSelect(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path fill="currentColor" d="M9 16.2 4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4z" /></svg
			>
			{#if selected}
				Deselect
			{:else}
				Select
			{/if}
		</DropdownMenu.Item>
	{/if}
	{#if onTogglePin && !archived}
		<DropdownMenu.Item onclick={() => onTogglePin(note)}>
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
		</DropdownMenu.Item>
	{/if}
	<DropdownMenu.Item onclick={() => onRename(note)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
			/></svg
		>
		Change title
	</DropdownMenu.Item>
	{#if !archived}
		<DropdownMenu.Item onclick={() => onTags(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M10.537 2.164a3 3 0 0 1 2.244.727l.15.14 7.822 7.823a3 3 0 0 1 .135 4.098l-.135.144-5.657 5.657a3 3 0 0 1-4.098.135l-.144-.135L3.03 12.93a3 3 0 0 1-.878-2.188l.011-.205.472-5.185a3 3 0 0 1 2.537-2.695l.179-.021zM8.024 8.025a2 2 0 1 0 2.829 2.829 2 2 0 0 0-2.829-2.829"
				/></svg
			>
			Tags…
		</DropdownMenu.Item>
	{/if}
	{#if archived}
		<DropdownMenu.Item onclick={() => onRestore(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M2.614 5.426A1.5 1.5 0 0 1 4 4.5h10a7.5 7.5 0 1 1 0 15H5a1.5 1.5 0 0 1 0-3h9a4.5 4.5 0 1 0 0-9H7.621l.94.94a1.5 1.5 0 0 1-2.122 2.12l-3.5-3.5a1.5 1.5 0 0 1-.325-1.634Z"
				/></svg
			>
			Restore
		</DropdownMenu.Item>
	{:else}
		<DropdownMenu.Item onclick={() => onArchive(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M7.414 3A2 2 0 0 0 6 3.586L3.586 6a2 2 0 0 0-.543 1h17.914a2 2 0 0 0-.543-1L18 3.586A2 2 0 0 0 16.586 3zM21 9H3v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2zm-9 2a1 1 0 0 1 1 1v3.186l.414-.415a1 1 0 0 1 1.414 1.415l-2.12 2.121a1 1 0 0 1-1.415 0l-2.121-2.121a1 1 0 0 1 1.414-1.415l.414.415V12a1 1 0 0 1 1-1"
				/></svg
			>
			Archive
		</DropdownMenu.Item>
	{/if}
	<DropdownMenu.Item onclick={() => onInfo(note)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m-.01 8H11a1 1 0 0 0-.117 1.993L11 12v4.99c0 .52.394.95.9 1.004l.11.006h.49a1 1 0 0 0 .596-1.803L13 16.134V11.01c0-.52-.394-.95-.9-1.004zM12 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2"
			/></svg
		>
		Get info
	</DropdownMenu.Item>
	{#if !archived}
		<DropdownMenu.Item onclick={() => onHistory(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M11.463 2.891a1.5 1.5 0 0 1-1.131 1.795c-.185.042-.367.09-.546.146A1.5 1.5 0 1 1 8.9 1.965c.252-.078.508-.146.767-.205a1.5 1.5 0 0 1 1.795 1.131m1.074 0a1.5 1.5 0 0 1 1.795-1.13C19.008 2.82 22.5 7 22.5 12c0 5.799-4.701 10.5-10.5 10.5-4.999 0-9.179-3.492-10.24-8.168a1.5 1.5 0 0 1 2.926-.664 7.5 7.5 0 1 0 8.982-8.982 1.5 1.5 0 0 1-1.13-1.795M6.98 4.381A1.5 1.5 0 0 1 6.9 6.5a7.555 7.555 0 0 0-.4.4 1.5 1.5 0 1 1-2.2-2.04c.18-.194.367-.38.56-.56a1.5 1.5 0 0 1 2.12.08M12 5.5A1.5 1.5 0 0 1 13.5 7v4.379l2.56 2.56a1.5 1.5 0 1 1-2.12 2.122l-3-3A1.5 1.5 0 0 1 10.5 12V7A1.5 1.5 0 0 1 12 5.5M3.84 7.91a1.5 1.5 0 0 1 .992 1.876c-.055.179-.104.361-.146.546a1.5 1.5 0 0 1-2.926-.664c.059-.26.127-.515.205-.767a1.5 1.5 0 0 1 1.876-.991"
				/></svg
			>
			Version history
		</DropdownMenu.Item>
		<DropdownMenu.Sub>
			<DropdownMenu.SubTrigger>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M12 2v6.5a1.5 1.5 0 0 0 1.5 1.5H20v5.757l-1.293-1.293a1 1 0 1 0-1.414 1.415L18.414 17H14a1 1 0 1 0 0 2h4.414l-1.121 1.121a1 1 0 0 0 1.414 1.415l1.276-1.277A2 2 0 0 1 18 22H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2zm8 13.757 1.535 1.536a1 1 0 0 1 0 1.414L20 20.243zM14 2.043a2 2 0 0 1 1 .543L19.414 7a2 2 0 0 1 .543 1H14z"
					/></svg
				>
				Export
			</DropdownMenu.SubTrigger>
			<DropdownMenu.SubContent class="w-40">
				<DropdownMenu.Item onclick={() => onExport(note, 'md')}>Markdown (.md)</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => onExport(note, 'html')}>HTML (.html)</DropdownMenu.Item>
			</DropdownMenu.SubContent>
		</DropdownMenu.Sub>
		<DropdownMenu.Item onclick={() => onConvertToTask(note)}>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M7.416 3A4.983 4.983 0 0 0 7 5a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2c0-.711-.148-1.388-.416-2H18a2 2 0 0 1 2 2v15a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zM12 14H9a1 1 0 1 0 0 2h3a1 1 0 1 0 0-2m3-4H9a1 1 0 0 0-.117 1.993L9 12h6a1 1 0 1 0 0-2m-3-8a2.99 2.99 0 0 1 2.236 1c.428.478.704 1.093.755 1.772L15 5H9c0-.725.257-1.39.685-1.908L9.764 3c.55-.614 1.348-1 2.236-1"
				/></svg
			>
			Convert to task
		</DropdownMenu.Item>
	{/if}
	<DropdownMenu.Separator />
	<DropdownMenu.Item variant="destructive" onclick={() => onDelete(note)}>
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
			><path
				fill="currentColor"
				d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
			/></svg
		>
		Delete
	</DropdownMenu.Item>
</DropdownMenu.Content>
