<script lang="ts">
	import { onMount } from 'svelte';
	import { notesInvoke } from '$lib/notes/liveNotes';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Card, CardContent } from '$lib/components/ui/card/index.js';

	type NoteDetails = {
		kind: string; // "note" | "folder"
		name: string;
		path: string;
		size_bytes: number;
		created: number;
		modified: number;
		word_count: number;
		note_count: number;
		folder_count: number;
	};

	let open = $state(false);
	let info = $state<NoteDetails | null>(null);
	let error = $state<string | null>(null);

	// opened from the sidebar context menus
	function handleOpen(e: Event) {
		const path = (e as CustomEvent<{ path: string }>).detail?.path;
		if (!path) return;
		open = true;
		info = null;
		error = null;
		void notesInvoke<NoteDetails>('note_info', { path })
			.then((d) => (info = d))
			.catch((err) => (error = String(err)));
	}

	onMount(() => {
		window.addEventListener('open-note-info-dialog', handleOpen);
		return () => window.removeEventListener('open-note-info-dialog', handleOpen);
	});

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function formatDate(ms: number): string {
		if (!ms) return '—';
		return new Date(ms).toLocaleString(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short'
		});
	}

	// short display path: folder names only, not the absolute location
	function shortPath(path: string, kind: string): string {
		const withoutFile = kind === 'note' ? path.split('/').slice(0, -1).join('/') : path;
		return withoutFile || '/';
	}

	// cap long names like the editor title does so the dialog never breaks
	const NAME_LIMIT = 50;
	const displayName = $derived(
		info && info.name.length > NAME_LIMIT
			? `${info.name.slice(0, NAME_LIMIT)}…`
			: (info?.name ?? '')
	);
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">Info</Dialog.Title>
		<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
			<span class="text-[13px] font-medium text-foreground">
				{info?.kind === 'folder' ? 'Folder Info' : 'Note Info'}
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
		{#if error}
			<div class="px-4 pb-5 text-[12px] text-destructive sm:px-5">{error}</div>
		{:else if !info}
			<div class="px-4 pb-5 text-[12px] text-muted-foreground sm:px-5">Loading…</div>
		{:else}
			<div class="flex flex-col gap-1 px-4 pb-5 sm:px-5">
				<div class="mb-2 flex min-w-0 items-center gap-2.5">
					<svg
						class="shrink-0 text-muted-foreground"
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						><path
							fill="currentColor"
							d={info.kind === 'folder'
								? 'M21.328 10a.5.5 0 0 1 .496.563l-.017.08-2.89 9.644a1 1 0 0 1-.84.706L17.96 21H4a1.99 1.99 0 0 1-1.099-.328.494.494 0 0 1-.026-.234l.017-.082 2.894-9.643a1 1 0 0 1 .839-.706L6.744 10zM9.52 3a2 2 0 0 1 1.443.614l.12.137L12.48 5.5H19a2 2 0 0 1 1.995 1.85L21 7.5V8H6.744A3 3 0 0 0 3.93 9.96l-.06.178L2 16.37V5a2 2 0 0 1 1.85-1.995L4 3z'
								: 'M18 2a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2zm-6 11H9a1 1 0 1 0 0 2h3a1 1 0 1 0 0-2m3-5H9a1 1 0 0 0-.117 1.993L9 10h6a1 1 0 0 0 .117-1.993z'}
						/></svg
					>
					<span
						class="min-w-0 flex-1 truncate text-[14px] font-semibold tracking-tight text-foreground"
						title={info.name}
					>
						{displayName}
					</span>
				</div>
				<Card size="sm" class="gap-0 py-0">
					<CardContent class="grid gap-0.5 px-3 py-1 text-[12px]">
						<div class="flex items-center justify-between gap-4 py-1">
							<span class="text-muted-foreground">Size</span>
							<span class="text-foreground tabular-nums">{formatSize(info.size_bytes)}</span>
						</div>
						<div class="flex items-center justify-between gap-4 py-1">
							<span class="text-muted-foreground">Created</span>
							<span class="text-foreground">{formatDate(info.created)}</span>
						</div>
						<div class="flex items-center justify-between gap-4 py-1">
							<span class="text-muted-foreground">Modified</span>
							<span class="text-foreground">{formatDate(info.modified)}</span>
						</div>
						{#if info.kind === 'note'}
							<div class="flex items-center justify-between gap-4 py-1">
								<span class="text-muted-foreground">Words</span>
								<span class="text-foreground tabular-nums">{info.word_count}</span>
							</div>
						{:else}
							<div class="flex items-center justify-between gap-4 py-1">
								<span class="text-muted-foreground">Notes</span>
								<span class="text-foreground tabular-nums">{info.note_count}</span>
							</div>
							<div class="flex items-center justify-between gap-4 py-1">
								<span class="text-muted-foreground">Subfolders</span>
								<span class="text-foreground tabular-nums">{info.folder_count}</span>
							</div>
						{/if}
						<div class="mt-1 flex flex-col gap-1 border-t border-border pt-2">
							<span class="text-muted-foreground">Location</span>
							<span class="truncate text-foreground/80" title={info.path}>
								{shortPath(info.path, info.kind)}
							</span>
						</div>
					</CardContent>
				</Card>
			</div>
		{/if}
	</Dialog.Content>
</Dialog.Root>
