<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';
	import { mingcuteNames, loadIconBodies } from '$lib/notes/icons';

	// how many icons render at once; the full mingcute set is too large to draw
	const LIMIT = 120;

	let open = $state(false);
	let kind = $state<'note' | 'folder'>('note');
	let notePath = $state<string | null>(null);
	let folderRel = $state<string | null>(null);
	let query = $state('');
	let names = $state<string[]>([]);
	let entries = $state<{ name: string; body: string }[]>([]);
	let loading = $state(false);
	let failed = $state(false);
	let inputEl = $state<HTMLInputElement | null>(null);

	const current = $derived(
		kind === 'note'
			? notePath
				? notesState.noteIcons[notePath]
				: null
			: folderRel
				? notesState.folderIcons[folderRel]
				: null
	);

	onMount(() => {
		const handler = (event: Event) => {
			const detail = (event as CustomEvent).detail ?? {};
			kind = detail.kind === 'folder' ? 'folder' : 'note';
			notePath = detail.path ?? null;
			folderRel = detail.rel ?? null;
			query = '';
			open = true;
			requestAnimationFrame(() => inputEl?.focus());
			void load();
		};
		window.addEventListener('open-note-icon-picker', handler);
		return () => window.removeEventListener('open-note-icon-picker', handler);
	});

	// filtered names, then bodies fetched in batches until the grid is full;
	// names the api cannot resolve are skipped so no blank tile is ever shown
	$effect(() => {
		if (!open || names.length === 0) return;
		const q = query.trim().toLowerCase();
		const list = q ? names.filter((name) => name.includes(q)) : names;
		let cancelled = false;
		loading = true;
		failed = false;
		const timer = setTimeout(async () => {
			try {
				const picked = await pickVisible(list);
				if (!cancelled) entries = picked;
			} catch {
				if (!cancelled) failed = true;
			} finally {
				if (!cancelled) loading = false;
			}
		}, 150);
		return () => {
			cancelled = true;
			clearTimeout(timer);
		};
	});

	async function pickVisible(list: string[]): Promise<{ name: string; body: string }[]> {
		const out: { name: string; body: string }[] = [];
		let cursor = 0;
		while (out.length < LIMIT && cursor < list.length) {
			const chunk = list.slice(cursor, cursor + LIMIT);
			cursor += chunk.length;
			const bodies = await loadIconBodies(chunk);
			for (const name of chunk) {
				const body = bodies[name];
				if (body) out.push({ name, body });
				if (out.length >= LIMIT) break;
			}
		}
		return out;
	}

	async function load() {
		if (names.length > 0) return;
		loading = true;
		failed = false;
		try {
			names = await mingcuteNames();
		} catch {
			failed = true;
		} finally {
			loading = false;
		}
	}

	function pick(icon: string | null) {
		if (kind === 'note' && notePath) notesState.setNoteIcon(notePath, icon);
		else if (kind === 'folder' && folderRel) notesState.setFolderIcon(folderRel, icon);
		open = false;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-md gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="px-5 pt-5 text-[15px] font-semibold">Change icon</Dialog.Title>
		<Dialog.Description class="px-5 pt-1 text-[12px] text-muted-foreground">
			Search MingCute icons by name.
		</Dialog.Description>
		<div class="px-5 pt-4">
			<Input bind:ref={inputEl} bind:value={query} placeholder="Search icons…" spellcheck="false" />
		</div>
		<ScrollArea class="mt-3 max-h-72">
			<div class="px-5">
				<div class="grid grid-cols-6 gap-1.5 pb-4 sm:grid-cols-8">
					{#each entries as entry (entry.name)}
						<button
							type="button"
							aria-label={entry.name}
							title={entry.name}
							class="flex aspect-square items-center justify-center rounded-md border border-border/60 text-muted-foreground transition-colors hover:border-border hover:bg-muted/50 hover:text-foreground"
							onclick={() => pick(`mingcute:${entry.name}`)}
						>
							<svg
								viewBox="0 0 24 24"
								width="20"
								height="20"
								fill="currentColor"
								aria-hidden="true"
							>
								<!-- sanitized by the icon loader before it reaches the dom -->
								<!-- eslint-disable-next-line svelte/no-at-html-tags -->
								{@html entry.body}
							</svg>
						</button>
					{/each}
				</div>
				{#if loading && entries.length === 0}
					<p class="pb-4 text-[12px] text-muted-foreground">Loading icons…</p>
				{:else if failed}
					<p class="pb-4 text-[12px] text-red-400/90">Could not load icons.</p>
				{:else if entries.length === 0}
					<p class="pb-4 text-[12px] text-muted-foreground">No icons match.</p>
				{/if}
			</div>
		</ScrollArea>
		<div class="flex items-center justify-between gap-2 border-t border-border/40 px-5 py-3">
			<Button
				type="button"
				variant="ghost"
				size="sm"
				disabled={!current}
				onclick={() => pick(null)}
			>
				Remove icon
			</Button>
			<span class="text-[11px] text-muted-foreground/60">{names.length} icons</span>
		</div>
	</Dialog.Content>
</Dialog.Root>
