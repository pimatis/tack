<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';

	let open = $state(false);
	let notePath = $state<string | null>(null);
	let tags = $state<string[]>([]);
	let draft = $state('');
	let inputEl = $state<HTMLInputElement | null>(null);

	onMount(() => {
		const handler = (event: Event) => {
			const detail = (event as CustomEvent).detail ?? {};
			notePath = detail.path ?? null;
			tags = notePath ? [...notesState.tagsOfNote(notePath)] : [];
			draft = '';
			open = true;
			requestAnimationFrame(() => inputEl?.focus());
		};
		window.addEventListener('open-note-tags-dialog', handler);
		return () => window.removeEventListener('open-note-tags-dialog', handler);
	});

	function add() {
		const tag = draft.trim().replace(/^#/, '');
		if (!tag || !notePath || tags.includes(tag)) {
			draft = '';
			return;
		}
		void notesState.addTagToNote(notePath, tag);
		tags = [...tags, tag];
		draft = '';
	}

	function remove(tag: string) {
		if (!notePath) return;
		void notesState.removeTagFromNote(notePath, tag);
		tags = tags.filter((t) => t !== tag);
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="max-w-sm gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="px-5 pt-5 text-[15px] font-semibold">Note tags</Dialog.Title>
		<Dialog.Description class="px-5 pt-1 text-[12px] text-muted-foreground">
			Tags are stored in the note's frontmatter.
		</Dialog.Description>
		<form
			onsubmit={(e) => {
				e.preventDefault();
				add();
			}}
			class="flex gap-2 px-5 pt-4"
		>
			<Input bind:ref={inputEl} bind:value={draft} placeholder="Add a tag…" spellcheck="false" />
			<Button type="submit" size="sm" variant="outline">Add</Button>
		</form>
		<div class="flex flex-wrap gap-1.5 px-5 py-4">
			{#each tags as tag (tag)}
				<span
					class="flex items-center gap-1 rounded-full border border-border bg-muted/40 px-2.5 py-1 text-[12px]"
				>
					#{tag}
					<button
						type="button"
						class="text-muted-foreground/70 transition-colors hover:text-foreground"
						aria-label="Remove tag {tag}"
						onclick={() => remove(tag)}
					>
						×
					</button>
				</span>
			{:else}
				<p class="text-[12px] text-muted-foreground">No tags yet.</p>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>
