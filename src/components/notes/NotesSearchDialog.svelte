<script lang="ts">
	import * as Command from '$lib/components/ui/command/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';
	import { searchNotes, type NoteSearchResult } from '$lib/notes/search';
	import { isTauri } from '$lib/db/client';

	let open = $state(false);
	let query = $state('');
	// null = no query yet, show the plain note list; [] = searched, nothing found
	let results = $state<NoteSearchResult[] | null>(null);

	$effect(() => {
		const handleOpen = () => (open = true);
		window.addEventListener('open-notes-search', handleOpen);
		return () => window.removeEventListener('open-notes-search', handleOpen);
	});

	$effect(() => {
		if (open) void notesState.refresh();
	});

	// debounced full-text search over note names and content
	$effect(() => {
		const q = query.trim();
		if (!open || !q) {
			results = null;
			return;
		}
		const timer = setTimeout(async () => {
			results = await searchNotes(q).catch(() => []);
		}, 150);
		return () => clearTimeout(timer);
	});

	function selectNote(path: string) {
		notesState.activeTab = 'notes';
		void notesState.openNote(path);
		open = false;
	}
</script>

<Command.Dialog
	bind:open
	shouldFilter={false}
	title="Search notes"
	description="Search notes by title and content"
	showCloseButton={false}
	class="top-[12%]! w-[calc(100vw-2rem)]! max-w-[560px]! sm:top-[18%]"
>
	<Command.Input bind:value={query} placeholder="Search notes..." />
	<Command.List class="max-h-[60vh] sm:max-h-[400px]">
		<Command.Empty>
			{!isTauri()
				? 'Notes are only available in the desktop app.'
				: !notesState.folder
					? 'Choose a notes folder first.'
					: results
						? 'No matching notes.'
						: 'No notes found.'}
		</Command.Empty>

		{#if isTauri() && notesState.folder && results}
			<!-- full-text matches with a content snippet -->
			<Command.Group heading="Results">
				{#each results as note (note.path)}
					<Command.Item
						value={note.name}
						onSelect={() => selectNote(note.path)}
						class="[&_.cn-command-item-indicator]:hidden"
					>
						<svg
							class="mt-0.5 text-muted-foreground"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5zM8 12h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2m0 4h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2"
							/></svg
						>
						<span class="flex min-w-0 flex-col">
							<span class="truncate">{note.name.replace(/\.md$/, '')}</span>
							{#if note.snippet}
								<span class="truncate text-[11px] text-muted-foreground">{note.snippet}</span>
							{/if}
						</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{:else if isTauri() && notesState.folder && notesState.notes.length > 0}
			<Command.Group heading="Notes">
				{#each notesState.notes as note (note.path)}
					<Command.Item
						value={note.name}
						onSelect={() => selectNote(note.path)}
						class="[&_.cn-command-item-indicator]:hidden"
					>
						<svg
							class="text-muted-foreground"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5zM8 12h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2m0 4h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2"
							/></svg
						>
						<span class="min-w-0 flex-1 truncate">{note.name.replace(/\.md$/, '')}</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
