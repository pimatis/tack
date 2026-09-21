<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';
	import type { NoteInfo } from '$lib/notes/notesState.svelte';

	let searchQuery = $state('');

	let filtered = $derived.by(() => {
		const q = searchQuery.toLowerCase().trim();
		if (!q) return notesState.archived;
		return notesState.archived.filter((n) => n.name.toLowerCase().includes(q));
	});

	function formatDate(ms: number): string {
		const d = new Date(ms);
		const now = new Date();
		const diffMin = Math.floor((now.getTime() - d.getTime()) / 60000);
		const diffHr = Math.floor(diffMin / 60);
		const diffDay = Math.floor(diffHr / 24);
		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		if (diffHr < 24) return `${diffHr}h ago`;
		if (diffDay < 7) return `${diffDay}d ago`;
		return new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric' }).format(d);
	}

	// restore puts the note back into the main notes folder
	async function handleRestore(note: NoteInfo) {
		await notesState.restoreNote(note.path);
	}

	// deleting from archive moves the note to the notes trash
	async function handleDelete(note: NoteInfo) {
		await notesState.deleteNote(note.path);
	}

	function openNote(path: string) {
		notesState.activeTab = 'notes';
		void notesState.openNote(path);
		void goto('/');
	}

	// right-click on a note opens the dropdown at the pointer
	let menuOpen = $state(false);
	let menuX = $state(0);
	let menuY = $state(0);
	let menuNote = $state<NoteInfo | null>(null);

	function openMenu(event: MouseEvent, note: NoteInfo) {
		event.preventDefault();
		menuNote = note;
		menuX = event.clientX;
		menuY = event.clientY;
		menuOpen = true;
	}
</script>

<section class="flex h-full flex-col">
	<!-- header -->
	<header
		class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-6 sm:py-4"
	>
		<div class="min-w-0">
			<div class="flex items-center gap-3">
				<h1 class="text-base font-semibold tracking-tight sm:text-lg">Archive</h1>
				{#if notesState.archived.length > 0}
					<div class="flex items-center gap-1.5 text-[12px] text-muted-foreground">
						<span class="size-1.5 rounded-full bg-foreground/30"></span>
						<span
							>{searchQuery.trim() ? filtered.length : notesState.archived.length}
							{(searchQuery.trim() ? filtered.length : notesState.archived.length) === 1
								? 'note'
								: 'notes'}</span
						>
					</div>
				{/if}
			</div>
			<p class="truncate text-xs text-muted-foreground sm:text-sm">
				Archived notes can be restored or moved to the trash
			</p>
		</div>
		<Button variant="ghost" size="sm" href="/">
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M16.06 10.94a1.5 1.5 0 0 1 0 2.12l-5.656 5.658a1.5 1.5 0 1 1-2.121-2.122L12.879 12 8.283 7.404a1.5 1.5 0 0 1 2.12-2.122l5.658 5.657Z"
				/></svg
			>
			Back
		</Button>
	</header>

	<!-- content -->
	<div class="flex-1 overflow-y-auto px-4 py-4 sm:px-6 sm:py-6">
		<div class="mx-auto max-w-3xl">
			{#if notesState.loading}
				<div class="flex items-center justify-center py-20 text-[13px] text-muted-foreground">
					Loading archive...
				</div>
			{:else if notesState.error}
				<div class="flex flex-col items-center gap-3 py-20">
					<p class="text-[13px] text-destructive" role="alert">{notesState.error}</p>
					<Button variant="outline" size="sm" onclick={() => void notesState.refresh()}>
						Try again
					</Button>
				</div>
			{:else if notesState.archived.length === 0}
				<div class="flex flex-col items-center justify-center gap-5 py-28">
					<div class="flex size-14 items-center justify-center rounded-2xl bg-muted/50">
						<svg
							class="text-muted-foreground/60"
							width="28"
							height="28"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M7.414 3A2 2 0 0 0 6 3.586L3.586 6a2 2 0 0 0-.543 1h17.914a2 2 0 0 0-.543-1L18 3.586A2 2 0 0 0 16.586 3zM21 9H3v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2zm-9 2a1 1 0 0 1 1 1v3.186l.414-.415a1 1 0 0 1 1.414 1.415l-2.12 2.121a1 1 0 0 1-1.415 0l-2.121-2.121a1 1 0 0 1 1.414-1.415l.414.415V12a1 1 0 0 1 1-1"
							/></svg
						>
					</div>
					<div class="flex flex-col items-center gap-1.5">
						<p class="text-[15px] font-semibold text-foreground">Nothing archived yet</p>
						<p class="text-[13px] text-muted-foreground">Archived notes will appear here</p>
					</div>
				</div>
			{:else}
				<!-- search -->
				<div class="relative mb-4">
					<svg
						class="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground/50"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
						/></svg
					>
					<Input
						placeholder="Search archived notes..."
						bind:value={searchQuery}
						class="h-8 w-full rounded-lg border border-input bg-transparent pr-3 pl-8 text-[13px] text-foreground transition-all outline-none placeholder:text-muted-foreground/50 dark:bg-input/30"
					/>
				</div>

				<Separator class="mb-4" />

				{#if filtered.length === 0}
					<div class="flex flex-col items-center gap-3 py-16">
						<p class="text-[13px] text-muted-foreground">No matching notes.</p>
						<Button
							variant="ghost"
							size="sm"
							class="h-auto p-0 text-[12px] font-medium text-foreground/70 transition-colors hover:text-foreground"
							onclick={() => (searchQuery = '')}
						>
							Clear search
						</Button>
					</div>
				{:else}
					<!-- archived note list -->
					<div class="flex flex-col">
						{#each filtered as note (note.path)}
							<button
								type="button"
								class="group/note -mx-2 flex items-center gap-2.5 rounded-lg px-2 py-2 text-left transition-colors hover:bg-muted/40"
								onclick={() => openNote(note.path)}
								oncontextmenu={(e) => openMenu(e, note)}
							>
								<!-- file icon -->
								<span class="flex size-5 shrink-0 items-center justify-center">
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
								</span>

								<!-- type badge -->
								<span class="shrink-0 font-mono text-[11px] font-medium text-muted-foreground/50">
									NOTE
								</span>

								<!-- title -->
								<span class="min-w-0 flex-1 truncate text-[13px] text-foreground/70">
									{note.name.replace(/\.md$/, '')}
								</span>

								<!-- archived date -->
								<span
									class="hidden w-16 shrink-0 text-right text-[11px] text-muted-foreground/40 sm:block"
								>
									{formatDate(note.modified)}
								</span>
							</button>
						{/each}
					</div>
					<DropdownMenu.Root bind:open={menuOpen}>
						<DropdownMenu.Trigger
							class="h-0 w-0 outline-none"
							style="position: fixed; left: {menuX}px; top: {menuY}px"
						/>
						<DropdownMenu.Content class="w-44">
							{#if menuNote}
								<DropdownMenu.Item onclick={() => void handleRestore(menuNote!)}>
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
										><path
											fill="currentColor"
											d="M2.614 5.426A1.5 1.5 0 0 1 4 4.5h10a7.5 7.5 0 1 1 0 15H5a1.5 1.5 0 0 1 0-3h9a4.5 4.5 0 1 0 0-9H7.621l.94.94a1.5 1.5 0 0 1-2.122 2.12l-3.5-3.5a1.5 1.5 0 0 1-.325-1.634Z"
										/></svg
									>
									Restore
								</DropdownMenu.Item>
								<DropdownMenu.Separator />
								<DropdownMenu.Item
									variant="destructive"
									onclick={() => void handleDelete(menuNote!)}
								>
									<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
										><path
											fill="currentColor"
											d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
										/></svg
									>
									Move to trash
								</DropdownMenu.Item>
							{/if}
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				{/if}
			{/if}
		</div>
	</div>
</section>
