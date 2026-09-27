<script lang="ts">
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import type { NoteInfo } from '$lib/notes/notesState.svelte';
	import MingcuteIcon from './MingcuteIcon.svelte';

	let {
		note,
		archived = false,
		pinned = false,
		selected = false,
		onRename,
		onChangeIcon,
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
		onChangeIcon: (note: NoteInfo) => void;
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
			<MingcuteIcon icon="check-fill" size={16} />
			{#if selected}
				Deselect
			{:else}
				Select
			{/if}
		</DropdownMenu.Item>
	{/if}
	{#if onTogglePin && !archived}
		<DropdownMenu.Item onclick={() => onTogglePin(note)}>
			<MingcuteIcon icon="pin-fill" size={16} />
			{#if pinned}
				Unpin
			{:else}
				Pin
			{/if}
		</DropdownMenu.Item>
	{/if}
	<DropdownMenu.Item onclick={() => onRename(note)}>
		<MingcuteIcon icon="edit-2-fill" size={16} />
		Change title
	</DropdownMenu.Item>
	<DropdownMenu.Item onclick={() => onChangeIcon(note)}>
		<MingcuteIcon icon="palette-fill" size={16} />
		Change icon
	</DropdownMenu.Item>
	{#if !archived}
		<DropdownMenu.Item onclick={() => onTags(note)}>
			<MingcuteIcon icon="tag-fill" size={16} />
			Tags…
		</DropdownMenu.Item>
	{/if}
	{#if archived}
		<DropdownMenu.Item onclick={() => onRestore(note)}>
			<MingcuteIcon icon="restore-fill" size={16} />
			Restore
		</DropdownMenu.Item>
	{:else}
		<DropdownMenu.Item onclick={() => onArchive(note)}>
			<MingcuteIcon icon="archive-fill" size={16} />
			Archive
		</DropdownMenu.Item>
	{/if}
	<DropdownMenu.Item onclick={() => onInfo(note)}>
		<MingcuteIcon icon="information-fill" size={16} />
		Get info
	</DropdownMenu.Item>
	{#if !archived}
		<DropdownMenu.Item onclick={() => onHistory(note)}>
			<MingcuteIcon icon="history-fill" size={16} />
			Version history
		</DropdownMenu.Item>
		<DropdownMenu.Sub>
			<DropdownMenu.SubTrigger>
				<MingcuteIcon icon="file-export-fill" size={16} />
				Export
			</DropdownMenu.SubTrigger>
			<DropdownMenu.SubContent class="w-40">
				<DropdownMenu.Item onclick={() => onExport(note, 'md')}>Markdown (.md)</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => onExport(note, 'html')}>HTML (.html)</DropdownMenu.Item>
			</DropdownMenu.SubContent>
		</DropdownMenu.Sub>
		<DropdownMenu.Item onclick={() => onConvertToTask(note)}>
			<MingcuteIcon icon="task-fill" size={16} />
			Convert to task
		</DropdownMenu.Item>
	{/if}
	<DropdownMenu.Separator />
	<DropdownMenu.Item variant="destructive" onclick={() => onDelete(note)}>
		<MingcuteIcon icon="delete-2-fill" size={16} />
		Delete
	</DropdownMenu.Item>
</DropdownMenu.Content>
