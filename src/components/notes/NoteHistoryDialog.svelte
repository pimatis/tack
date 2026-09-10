<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';

	let open = $state(false);
	let notePath = $state<string | null>(null);
	let snapshots = $state<{ name: string; path: string; modified: number }[]>([]);
	let selected = $state<string | null>(null);
	let preview = $state('');
	let confirmRestore = $state(false);

	const DateTimeFormat = new Intl.DateTimeFormat('tr-TR', {
		dateStyle: 'medium',
		timeStyle: 'short'
	});

	onMount(() => {
		const handler = (event: Event) => {
			const detail = (event as CustomEvent).detail ?? {};
			notePath = detail.path ?? null;
			selected = null;
			preview = '';
			confirmRestore = false;
			open = true;
			void load();
		};
		window.addEventListener('open-note-history-dialog', handler);
		return () => window.removeEventListener('open-note-history-dialog', handler);
	});

	async function load() {
		if (!notePath) return;
		// newest first
		snapshots = (await notesState.listHistory(notePath)).sort((a, b) => b.modified - a.modified);
	}

	async function pick(path: string) {
		selected = path;
		confirmRestore = false;
		preview = await notesState.readHistory(path).catch(() => '');
	}

	async function restore() {
		if (!notePath || !selected) return;
		await notesState.restoreHistorySnapshot(notePath, selected);
		open = false;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="max-w-2xl gap-0 p-0">
		<Dialog.Header class="px-5 pt-5 pb-0">
			<Dialog.Title class="text-[15px] font-semibold">Version history</Dialog.Title>
			<Dialog.Description class="text-[12px] text-muted-foreground">
				Autosaved snapshots of this note. Restoring keeps the current version in history too.
			</Dialog.Description>
		</Dialog.Header>
		<div class="grid max-h-[60vh] grid-cols-1 overflow-hidden sm:grid-cols-[220px_1fr]">
			<div class="max-h-40 overflow-y-auto border-border/60 p-2 sm:max-h-[60vh] sm:border-r">
				{#if snapshots.length === 0}
					<p class="px-2 py-3 text-[12px] text-muted-foreground">No snapshots yet.</p>
				{:else}
					{#each snapshots as snap (snap.path)}
						<button
							type="button"
							class="block w-full truncate rounded-md px-2 py-1.5 text-left text-[12px] transition-colors {selected ===
							snap.path
								? 'bg-muted text-foreground'
								: 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'}"
							onclick={() => void pick(snap.path)}
						>
							{DateTimeFormat.format(new Date(snap.modified))}
						</button>
					{/each}
				{/if}
			</div>
			<div class="max-h-[60vh] overflow-y-auto p-4">
				{#if selected}
					{#if confirmRestore}
						<div
							class="mb-3 flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-muted/30 p-3"
						>
							<span class="min-w-0 text-[12px]">Replace the note with this version?</span>
							<div class="flex shrink-0 gap-2">
								<Button variant="outline" size="sm" onclick={() => (confirmRestore = false)}
									>Cancel</Button
								>
								<Button variant="destructive" size="sm" onclick={() => void restore()}
									>Restore</Button
								>
							</div>
						</div>
					{:else}
						<Button
							variant="outline"
							size="sm"
							class="mb-3"
							onclick={() => (confirmRestore = true)}
						>
							Restore this version
						</Button>
					{/if}
					<pre
						class="text-[12px] leading-relaxed whitespace-pre-wrap text-foreground/90">{preview}</pre>
				{:else}
					<p class="text-[12px] text-muted-foreground">Pick a snapshot to preview it.</p>
				{/if}
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
