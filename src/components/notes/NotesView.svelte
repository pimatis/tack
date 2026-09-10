<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import MarkdownRenderer from '../MarkdownRenderer.svelte';
	import { notesState } from '$lib/notes/notesState.svelte';
	import { isTauri } from '$lib/db/client';
	import BoldIcon from '@lucide/svelte/icons/bold';
	import ItalicIcon from '@lucide/svelte/icons/italic';
	import UnderlineIcon from '@lucide/svelte/icons/underline';
	import StrikethroughIcon from '@lucide/svelte/icons/strikethrough';
	import CodeIcon from '@lucide/svelte/icons/code';
	import HighlighterIcon from '@lucide/svelte/icons/highlighter';
	import LinkIcon from '@lucide/svelte/icons/link';
	import StatusIcon from '../StatusIcon.svelte';
	import { TaskPageState } from '$lib/task/taskState.svelte';
	import type { Task } from '$lib/types/task';
	import { getShortcutRegistry } from '$lib/shortcuts/index.js';

	// flush pending edits when leaving the editor; Cmd/Ctrl+S saves immediately
	onMount(() => {
		void notesState.init();
		const unregisterSave = getShortcutRegistry().register({
			id: 'save-note',
			// fires while typing in the editor textarea
			allowInInput: true,
			run: () => notesState.flushPendingSave()
		});
		return () => {
			unregisterSave();
			notesState.flushPendingSave();
		};
	});

	const noteTitle = $derived(
		notesState.selectedPath?.split('/').pop()?.replace(/\.md$/, '') ?? null
	);

	// title draft follows the selected note; committing triggers the rename
	let titleDraft = $state('');
	let titleFocused = $state(false);
	const TITLE_LIMIT = 100;
	const displayTitle = $derived(
		titleDraft.length > TITLE_LIMIT ? `${titleDraft.slice(0, TITLE_LIMIT)}…` : titleDraft
	);
	$effect(() => {
		void noteTitle;
		titleDraft = noteTitle ?? '';
	});

	function commitTitle() {
		if (!noteTitle) return;
		const next = titleDraft.trim();
		if (!next || next === noteTitle) {
			titleDraft = noteTitle;
			return;
		}
		if (notesState.selectedPath) void notesState.renameNote(notesState.selectedPath, next);
	}

	// notion-style slash menu: "/" at line start opens the block menu anchored to the caret
	type Block = { label: string; hint: string; insert: string; preview: string; caret?: number };
	const BLOCKS: Block[] = [
		{ label: 'Heading 1', hint: 'Big section heading', insert: '# ', preview: 'H1' },
		{ label: 'Heading 2', hint: 'Medium section heading', insert: '## ', preview: 'H2' },
		{ label: 'Heading 3', hint: 'Small section heading', insert: '### ', preview: 'H3' },
		{ label: 'Bullet list', hint: 'Create a simple bullet list', insert: '- ', preview: '•' },
		{ label: 'Numbered list', hint: 'Create a list with numbering', insert: '1. ', preview: '1.' },
		{ label: 'Quote', hint: 'Capture a quote', insert: '> ', preview: '❝' },
		{
			label: 'Code block',
			hint: 'Capture a code snippet',
			insert: '```\n\n```',
			preview: '</>',
			caret: 4
		},
		{ label: 'Divider', hint: 'Visually divide sections', insert: '\n---\n', preview: '—' }
	];

	let editorEl = $state<HTMLTextAreaElement | null>(null);
	let slashOpen = $state(false);
	let slashStart = $state(0);
	let slashCaret = $state(0);
	let slashAnchor = $state({ x: 0, y: 0 });
	let blockFilter = $state('');
	let filteredBlocks = $derived(
		BLOCKS.filter((b) => b.label.toLowerCase().includes(blockFilter.toLowerCase().trim()))
	);

	function handleEditorInput(event: Event) {
		notesState.scheduleSave();
		const caret = (event.currentTarget as HTMLTextAreaElement).selectionStart ?? 0;
		updateSlashMenu(caret);
		if (slashOpen) {
			if (mentionOpen) mentionOpen = false;
		} else {
			updateMentionMenu(caret);
		}
	}

	function updateSlashMenu(caret: number) {
		const textarea = editorEl;
		if (!textarea) return;
		slashCaret = caret;
		const before = notesState.content.slice(0, caret);
		// the slash token must start its own line: "/word" right after start or a newline
		const match = before.match(/(?:^|\n)\/([^/\n]*)$/);
		if (!match) {
			if (slashOpen) slashOpen = false;
			return;
		}
		slashStart = caret - match[1].length - 1;
		blockFilter = match[1];

		// monospace editor: caret coordinates are predictable from line/column
		const beforeToken = notesState.content.slice(0, slashStart);
		const line = beforeToken.split('\n').length - 1;
		const col = beforeToken.length - (beforeToken.lastIndexOf('\n') + 1);
		const lineHeight = 21; // matches leading-[21px]
		const charWidth = 7.8; // 13px monospace
		slashAnchor = {
			x: Math.min(12 + col * charWidth, Math.max(textarea.clientWidth - 270, 0)),
			y: Math.min(
				12 + (line + 1) * lineHeight - textarea.scrollTop,
				Math.max(textarea.clientHeight - 240, 0)
			)
		};
		slashOpen = true;
	}

	function applyBlock(block: Block) {
		// remove the "/query" token and insert the markdown snippet in its place
		notesState.content =
			notesState.content.slice(0, slashStart) + block.insert + notesState.content.slice(slashCaret);
		slashOpen = false;
		void tick().then(() => {
			editorEl?.focus();
			const caret = slashStart + (block.caret ?? block.insert.length);
			editorEl?.setSelectionRange(caret, caret);
			notesState.scheduleSave();
		});
	}

	// @-mention menu: "@query" opens a task/note picker anchored to the caret;
	// picked items become @[label](task:ID) / @[label](note:PATH) links
	type MentionItem = { kind: 'task' | 'note'; label: string; href: string; task?: Task };
	let mentionOpen = $state(false);
	let mentionStart = $state(0);
	let mentionCaret = $state(0);
	let mentionAnchor = $state({ x: 0, y: 0 });
	let mentionFilter = $state('');
	const mentionItems = $derived.by(() => {
		const query = mentionFilter.toLowerCase().trim();
		const tasks: MentionItem[] = TaskPageState.get()
			.tasks.filter((t) => !t.deletedAt && t.title.toLowerCase().includes(query))
			.slice(0, 5)
			.map((t) => ({
				kind: 'task' as const,
				// brackets would break the markdown link label
				label: t.title.replace(/[[\]]/g, ''),
				href: `task:${t.id}`,
				task: t
			}));
		const notes: MentionItem[] = notesState.allNotes
			.filter((n) => n.name.replace(/\.md$/, '').toLowerCase().includes(query))
			.slice(0, 5)
			.map((n) => ({
				kind: 'note' as const,
				label: n.name.replace(/\.md$/, '').replace(/[[\]]/g, ''),
				href: `note:${encodeURIComponent(n.path)}`
			}));
		// interleave tasks and notes so both kinds stay visible within the cap
		const mixed: MentionItem[] = [];
		for (let i = 0; i < Math.max(tasks.length, notes.length); i++) {
			if (tasks[i]) mixed.push(tasks[i]);
			if (notes[i]) mixed.push(notes[i]);
		}
		return mixed.slice(0, 5);
	});

	function updateMentionMenu(caret: number) {
		const textarea = editorEl;
		if (!textarea) return;
		mentionCaret = caret;
		const before = notesState.content.slice(0, caret);
		// the mention token must not be mid-word: "@query" after start or whitespace
		const match = before.match(/(?:^|\s)@([^@\s]*)$/);
		if (!match) {
			if (mentionOpen) mentionOpen = false;
			return;
		}
		mentionStart = caret - match[1].length - 1;
		mentionFilter = match[1];
		if (TaskPageState.get().tasks.length === 0) void TaskPageState.get().refresh();

		// same caret math as the slash menu
		const beforeToken = notesState.content.slice(0, mentionStart);
		const line = beforeToken.split('\n').length - 1;
		const col = beforeToken.length - (beforeToken.lastIndexOf('\n') + 1);
		const lineHeight = 21; // matches leading-[21px]
		const charWidth = 7.8; // 13px monospace
		mentionAnchor = {
			x: Math.min(12 + col * charWidth, Math.max(textarea.clientWidth - 270, 0)),
			y: Math.min(
				12 + (line + 1) * lineHeight - textarea.scrollTop,
				Math.max(textarea.clientHeight - 240, 0)
			)
		};
		mentionOpen = true;
	}

	function applyMention(item: MentionItem) {
		// replace the "@query" token with the mention link
		notesState.content =
			notesState.content.slice(0, mentionStart) +
			`@[${item.label}](${item.href})` +
			notesState.content.slice(mentionCaret);
		mentionOpen = false;
		void tick().then(() => {
			editorEl?.focus();
			const caret = mentionStart + item.label.length + item.href.length + 5;
			editorEl?.setSelectionRange(caret, caret);
			notesState.scheduleSave();
		});
	}

	// preview clicks on mention links open the task dialog or the note
	async function openMention(href: string) {
		if (href.startsWith('note:')) {
			notesState.activeTab = 'notes';
			void notesState.openNote(decodeURIComponent(href.slice(5)));
			return;
		}
		if (href.startsWith('task:')) {
			const id = decodeURIComponent(href.slice(5));
			const state = TaskPageState.get();
			if (state.tasks.length === 0) await state.refresh();
			const task = state.tasks.find((t) => t.id === id);
			if (!task) return;
			notesState.activeTab = 'tasks';
			state.handleEdit(task);
		}
	}

	// notion-style selection effects, applied by wrapping the markdown selection
	const EFFECTS = [
		{ label: 'Bold', token: '**', icon: BoldIcon },
		{ label: 'Italic', token: '*', icon: ItalicIcon },
		{ label: 'Underline', token: '++', icon: UnderlineIcon },
		{ label: 'Strikethrough', token: '~~', icon: StrikethroughIcon },
		{ label: 'Inline code', token: '`', icon: CodeIcon },
		{ label: 'Highlight', token: '==', icon: HighlighterIcon }
	];

	// effects dropdown opens at the right-click position
	let effectsOpen = $state(false);
	let effectsAnchor = $state({ x: 0, y: 0 });

	function handleEditorContextMenu(event: MouseEvent) {
		event.preventDefault();
		const rect = editorEl?.getBoundingClientRect();
		if (!rect) return;
		effectsAnchor = { x: event.clientX - rect.left, y: event.clientY - rect.top };
		effectsOpen = true;
	}

	function applyInlineEffect(token: string) {
		const textarea = editorEl;
		if (!textarea) return;
		const start = textarea.selectionStart;
		const end = textarea.selectionEnd;
		const selected = notesState.content.slice(start, end);
		// selecting an already-wrapped text toggles the effect off
		const isWrapped =
			selected.length >= token.length * 2 && selected.startsWith(token) && selected.endsWith(token);
		if (isWrapped) {
			notesState.content =
				notesState.content.slice(0, start) +
				selected.slice(token.length, -token.length) +
				notesState.content.slice(end);
		} else {
			notesState.content =
				notesState.content.slice(0, start) +
				token +
				selected +
				token +
				notesState.content.slice(end);
		}
		void tick().then(() => {
			textarea.focus();
			const selStart = isWrapped ? start : start + token.length;
			const selEnd = isWrapped
				? start + selected.length - token.length * 2
				: selStart + selected.length;
			textarea.setSelectionRange(selStart, selEnd);
		});
		notesState.scheduleSave();
	}

	// wraps the selection as a link; the "url" placeholder stays selected so
	// the user can type the address straight away
	function insertLink() {
		const textarea = editorEl;
		if (!textarea) return;
		const start = textarea.selectionStart;
		const end = textarea.selectionEnd;
		const selected = notesState.content.slice(start, end) || 'link text';
		notesState.content =
			notesState.content.slice(0, start) + `[${selected}](url)` + notesState.content.slice(end);
		void tick().then(() => {
			textarea.focus();
			const urlStart = start + selected.length + 3;
			textarea.setSelectionRange(urlStart, urlStart + 3);
		});
		notesState.scheduleSave();
	}

	// checkbox clicks rewrite the raw markdown line and save
	function toggleTodoLine(line: number) {
		const lines = notesState.content.split('\n');
		lines[line] = lines[line].replace(
			/^(\s*[-*+]\s+)\[( |x|X)\]/,
			(_, _prefix: string, mark: string) => (mark === ' ' ? '[x]' : '[ ]')
		);
		notesState.content = lines.join('\n');
		notesState.scheduleSave();
	}

	// compact relative time for the recents cards
	function relativeTime(ms: number): string {
		const minutes = Math.floor((Date.now() - ms) / 60000);
		if (minutes < 1) return 'just now';
		if (minutes < 60) return `${minutes}m ago`;
		const hours = Math.floor(minutes / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		if (days < 7) return `${days}d ago`;
		return new Date(ms).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
	}
</script>

<section class="flex h-full flex-col px-3 py-3 sm:px-5 sm:py-5 lg:px-8 lg:py-8">
	<header class="flex flex-wrap items-center justify-between gap-2 pb-4 sm:pb-5">
		<div class="flex min-w-0 flex-1 items-center gap-3">
			{#if noteTitle}
				<!-- editable title: committing renames the underlying .md file;
				     display caps at 100 chars so long titles never break the ui -->
				<input
					value={titleFocused ? titleDraft : displayTitle}
					onfocus={() => (titleFocused = true)}
					onblur={(e) => {
						titleFocused = false;
						titleDraft = e.currentTarget.value;
						commitTitle();
					}}
					oninput={(e) => (titleDraft = e.currentTarget.value)}
					onkeydown={(e) => {
						if (e.key === 'Enter') e.currentTarget.blur();
						else if (e.key === 'Escape') {
							titleDraft = noteTitle;
							e.currentTarget.blur();
						}
					}}
					spellcheck="false"
					aria-label="Note title"
					style="width: {Math.min(Math.max(displayTitle.length, 4) + 1, 44)}ch"
					class="min-w-0 shrink rounded bg-transparent text-[18px] font-semibold tracking-tight text-foreground outline-none hover:bg-muted/40 focus:bg-muted/40 sm:text-[22px]"
				/>
			{:else}
				<h1 class="text-[18px] font-semibold tracking-tight text-foreground sm:text-[22px]">
					Notes
				</h1>
			{/if}
			{#if notesState.saving}
				<div class="flex items-center gap-1.5 text-[12px] text-muted-foreground">
					<span>Saving…</span>
				</div>
			{/if}
		</div>
		{#if noteTitle}
			<!-- edit/preview switch, styled like the task view switcher -->
			<div class="flex items-center gap-0.5 rounded-lg border border-border bg-muted/20 p-0.5">
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon-sm"
								class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {notesState.preview
									? 'text-muted-foreground hover:text-foreground'
									: 'bg-muted text-foreground'}"
								onclick={() => (notesState.preview = false)}
								aria-label="Edit view"
							>
								<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
									/></svg
								>
							</Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="bottom">Edit</Tooltip.Content>
				</Tooltip.Root>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon-sm"
								class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {notesState.preview
									? 'bg-muted text-foreground'
									: 'text-muted-foreground hover:text-foreground'}"
								onclick={() => {
									notesState.flushPendingSave();
									notesState.preview = true;
								}}
								aria-label="Preview view"
							>
								<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M12 5c5.5 0 9.7 4.2 11 7-1.3 2.8-5.5 7-11 7S2.3 14.8 1 12c1.3-2.8 5.5-7 11-7m0 2.5A4.5 4.5 0 1 0 16.5 12 4.5 4.5 0 0 0 12 7.5m0 2a2.5 2.5 0 1 1-2.5 2.5 2.5 2.5 0 0 1 2.5-2.5"
									/></svg
								>
							</Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="bottom">Preview</Tooltip.Content>
				</Tooltip.Root>
			</div>
		{/if}
	</header>
	{#if !isTauri()}
		<div class="flex flex-1 items-center justify-center text-[13px] text-muted-foreground">
			Notes are only available in the desktop app.
		</div>
	{:else if !notesState.folder}
		<!-- first run: choose where notes live -->
		<div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
			<svg class="text-muted-foreground/50" width="36" height="36" viewBox="0 0 24 24" fill="none"
				><path
					fill="currentColor"
					d="M4 4a2 2 0 0 1 2-2h4a2 2 0 0 1 1.7 1l.55.83a.5.5 0 0 0 .42.17H18a2 2 0 0 1 2 2v1H7.66a3 3 0 0 0-2.82 2L2.9 14.4A1 1 0 0 1 2 14V4m0 16a2 2 0 0 0 2 2h13.34a3 3 0 0 0 2.82-2l2.4-7.2A1 1 0 0 0 23.6 12a2 2 0 0 0-1.9-2H7.66a1 1 0 0 0-.94.67L3 20z"
				/></svg
			>
			<div>
				<p class="text-[14px] font-medium text-foreground">Choose a notes folder</p>
				<p class="mt-1 text-[12px] text-muted-foreground">
					Your notes are saved as markdown files in that folder.
				</p>
			</div>
			<Button size="sm" onclick={() => void notesState.pickFolder()}>Choose folder</Button>
		</div>
	{:else if !noteTitle}
		{#if notesState.recentNotes.length > 0}
			<!-- empty state: quick access to the most visited notes -->
			<div class="flex flex-1 flex-col justify-center px-6 pb-20">
				<div class="mx-auto w-full max-w-2xl">
					<p class="text-[14px] font-medium text-foreground">Jump back in</p>
					<p class="mt-0.5 text-[12px] text-muted-foreground">
						Your most visited notes, one click away.
					</p>
					<div class="mt-4 grid grid-cols-2 gap-3">
						{#each notesState.recentNotes as note (note.path)}
							<button
								type="button"
								class="flex min-w-0 flex-col gap-2.5 rounded-lg border border-border bg-card/40 p-4 text-left transition-colors hover:bg-muted/40"
								onclick={() => void notesState.openNote(note.path)}
							>
								<span class="flex min-w-0 items-center gap-2.5">
									<span
										class="flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-muted/30 text-muted-foreground"
									>
										<svg width="13" height="13" viewBox="0 0 24 24" fill="none"
											><path
												fill="currentColor"
												d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5zM8 12h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2m0 4h8a1 1 0 1 1 0 2H8a1 1 0 1 1 0-2"
											/></svg
										>
									</span>
									<span class="truncate text-[13px] font-medium text-foreground">
										{note.name.replace(/\.md$/, '')}
									</span>
								</span>
								<span class="text-[11px] text-muted-foreground">
									{relativeTime(note.modified)}
								</span>
							</button>
						{/each}
					</div>
				</div>
			</div>
		{:else}
			<div class="flex flex-1 items-center justify-center text-[13px] text-muted-foreground">
				Select a note from the sidebar
			</div>
		{/if}
	{:else if notesState.preview}
		<div class="min-h-0 flex-1 overflow-auto pl-1">
			<MarkdownRenderer
				content={notesState.content}
				onToggleLine={toggleTodoLine}
				onOpenMention={(href) => void openMention(href)}
			/>
		</div>
	{:else}
		<!-- editor; typing "/" at line start opens the notion-style block menu,
		     right-click on text opens the selection effects dropdown -->
		<div class="relative min-h-0 flex-1">
			<textarea
				bind:this={editorEl}
				bind:value={notesState.content}
				oninput={handleEditorInput}
				oncontextmenu={handleEditorContextMenu}
				spellcheck="false"
				placeholder="Write in markdown…"
				class="h-full w-full resize-none bg-transparent p-3 font-mono text-[13px] leading-[21px] text-foreground outline-none placeholder:text-muted-foreground/50"
			></textarea>
			<DropdownMenu.Root bind:open={effectsOpen}>
				<DropdownMenu.Trigger
					class="absolute h-0 w-0 outline-none"
					style="left: {effectsAnchor.x}px; top: {effectsAnchor.y}px"
					aria-label="Text formatting"
				/>
				<DropdownMenu.Content align="start" class="w-44">
					{#each EFFECTS as effect (effect.label)}
						<DropdownMenu.Item class="gap-2.5" onclick={() => applyInlineEffect(effect.token)}>
							<effect.icon class="size-4 shrink-0" />
							{effect.label}
						</DropdownMenu.Item>
					{/each}
					<DropdownMenu.Separator />
					<DropdownMenu.Item class="gap-2.5" onclick={insertLink}>
						<LinkIcon class="size-4 shrink-0" />
						Link
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
			<DropdownMenu.Root bind:open={slashOpen}>
				<DropdownMenu.Trigger
					class="absolute h-0 w-0 outline-none"
					style="left: {slashAnchor.x}px; top: {slashAnchor.y}px"
					aria-label="Markdown blocks"
				/>
				<DropdownMenu.Content class="w-66 p-0" collisionPadding={8}>
					<div class="flex items-center gap-2 border-b border-border px-3">
						<svg
							class="shrink-0 text-muted-foreground/50"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
							/></svg
						>
						<input
							bind:value={blockFilter}
							placeholder="Filter blocks..."
							spellcheck="false"
							class="h-9 w-full bg-transparent text-[13px] text-foreground outline-none placeholder:text-muted-foreground/50"
							onkeydown={(e) => {
								// enter picks the first match, like notion
								if (e.key === 'Enter') {
									e.preventDefault();
									if (filteredBlocks[0]) applyBlock(filteredBlocks[0]);
								}
							}}
						/>
					</div>
					{#each filteredBlocks as block (block.label)}
						<DropdownMenu.Item class="gap-2.5 py-1.5" onSelect={() => applyBlock(block)}>
							<span
								class="flex h-5 w-6 shrink-0 items-center justify-center rounded border border-border bg-muted/40 font-mono text-[10px] text-muted-foreground"
							>
								{block.preview}
							</span>
							<span class="flex min-w-0 flex-col">
								<span class="truncate text-[13px]">{block.label}</span>
								<span class="truncate text-[11px] text-muted-foreground">{block.hint}</span>
							</span>
						</DropdownMenu.Item>
					{/each}
					{#if filteredBlocks.length === 0}
						<div class="px-3 py-2 text-[12px] text-muted-foreground">No blocks found</div>
					{/if}
				</DropdownMenu.Content>
			</DropdownMenu.Root>
			<DropdownMenu.Root bind:open={mentionOpen}>
				<DropdownMenu.Trigger
					class="absolute h-0 w-0 outline-none"
					style="left: {mentionAnchor.x}px; top: {mentionAnchor.y}px"
					aria-label="Mentions"
				/>
				<DropdownMenu.Content class="w-64 p-0" collisionPadding={8}>
					<div class="flex items-center gap-2 border-b border-border px-3">
						<svg
							class="shrink-0 text-muted-foreground/50"
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							><path
								fill="currentColor"
								d="M2 10.5a8.5 8.5 0 1 1 15.176 5.262l3.652 3.652a1 1 0 0 1-1.414 1.414l-3.652-3.652A8.5 8.5 0 0 1 2 10.5M10.5 6a1 1 0 0 0 0 2 2.5 2.5 0 0 1 2.5 2.5 1 1 0 1 0 2 0A4.5 4.5 0 0 0 10.5 6"
							/></svg
						>
						<input
							bind:value={mentionFilter}
							placeholder="Search tasks and notes..."
							spellcheck="false"
							class="h-9 w-full bg-transparent text-[13px] text-foreground outline-none placeholder:text-muted-foreground/50"
							onkeydown={(e) => {
								// enter picks the first match, like notion
								if (e.key === 'Enter') {
									e.preventDefault();
									if (mentionItems[0]) applyMention(mentionItems[0]);
								}
							}}
						/>
					</div>
					<div class="p-1">
						{#each mentionItems as item (item.href)}
							<DropdownMenu.Item class="gap-2.5 py-1.5" onSelect={() => applyMention(item)}>
								{#if item.task}
									<StatusIcon status={item.task.status} size={14} />
								{:else}
									<svg
										class="shrink-0 text-muted-foreground"
										width="14"
										height="14"
										viewBox="0 0 24 24"
										fill="none"
										><path
											fill="currentColor"
											d="M6 2a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8.17a2 2 0 0 0-.59-1.42l-4.58-4.58A2 2 0 0 0 13.41 2zm7.5 1.13L18.87 8H13.5z"
										/></svg
									>
								{/if}
								<span class="truncate text-[13px]">{item.label}</span>
								<span
									class="ml-auto shrink-0 text-[10px] tracking-wide text-muted-foreground uppercase"
								>
									{item.kind}
								</span>
							</DropdownMenu.Item>
						{/each}
						{#if mentionItems.length === 0}
							<div class="px-3 py-2 text-[12px] text-muted-foreground">No matches</div>
						{/if}
					</div>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</div>
	{/if}
</section>
