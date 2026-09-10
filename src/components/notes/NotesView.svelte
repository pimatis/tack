<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import MarkdownRenderer from '../MarkdownRenderer.svelte';
	import { notesState } from '$lib/notes/notesState.svelte';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { isTauri } from '$lib/db/client';
	import { notesInvoke, noteAssetUrl } from '$lib/notes/liveNotes';
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
	import { onDbChanged, onNotesChanged } from '$lib/db/client';
	import { getBacklinks, getUnlinkedMentions, type Backlink } from '$lib/notes/backlinks';
	import { wikiToFileName } from '$lib/notes/links';
	import { create as createTaskRepo } from '$lib/repositories/task.repository';
	import MentionPreviewCard from './MentionPreviewCard.svelte';
	import { Input } from '$lib/components/ui/input/index.js';
	import * as ContextMenu from '$lib/components/ui/context-menu/index.js';
	import { sortableItem, type DragDropState } from '$lib/dnd';

	// flush pending edits when leaving the editor; Cmd/Ctrl+S saves immediately
	onMount(() => {
		void notesState.init();
		const unregisterSave = getShortcutRegistry().register({
			id: 'save-note',
			// fires while typing in the editor textarea
			allowInInput: true,
			run: () => notesState.flushPendingSave()
		});
		const unregisterFind = getShortcutRegistry().register({
			id: 'find-note',
			allowInInput: true,
			run: () => openFind()
		});
		const unregisterCloseTab = getShortcutRegistry().register({
			id: 'close-note-tab',
			// fires while typing in the editor, like a browser's Cmd/Ctrl+W
			allowInInput: true,
			run: () => {
				if (notesState.selectedPath) notesState.closeTab(notesState.selectedPath);
			}
		});
		const unregisterNextTab = getShortcutRegistry().register({
			id: 'next-note-tab',
			allowInInput: true,
			run: () => notesState.cycleTab(1)
		});
		const unregisterPrevTab = getShortcutRegistry().register({
			id: 'prev-note-tab',
			allowInInput: true,
			run: () => notesState.cycleTab(-1)
		});
		const unregisterFocus = getShortcutRegistry().register({
			id: 'focus-mode',
			allowInInput: true,
			run: () => (notesState.focusMode = !notesState.focusMode)
		});
		// note → task conversion from the sidebar context menu
		const convertHandler = (event: Event) => {
			const path = (event as CustomEvent).detail?.path;
			if (typeof path === 'string') void convertNoteToTask(path);
		};
		window.addEventListener('convert-note-to-task', convertHandler);
		// cli/external edits: the watcher emits notes-changed, the db writer
		// (pin changes) rides db-changed; both debounce into one sync
		let syncTimer: ReturnType<typeof setTimeout> | undefined;
		const requestSync = () => {
			clearTimeout(syncTimer);
			syncTimer = setTimeout(() => void notesState.syncExternal(), 800);
		};
		const unlistenNotes = onNotesChanged(requestSync);
		const unlistenDb = onDbChanged(() => void notesState.reloadPins());
		return () => {
			unlistenNotes();
			unlistenDb();
			unregisterSave();
			unregisterFind();
			unregisterCloseTab();
			unregisterNextTab();
			unregisterPrevTab();
			unregisterFocus();
			window.removeEventListener('convert-note-to-task', convertHandler);
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
		// navigating must close the hover card, the cursor can stay on the same spot
		dismissMentionPreview();
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

	// ---- outline (table of contents) ----
	// mirrors render.ts: headings get sequential ids in document order
	type OutlineEntry = { id: string; level: number; text: string; line: number };
	let outlineOpen = $state(false);
	const outline = $derived.by(() => {
		const entries: OutlineEntry[] = [];
		const lines = notesState.content.split('\n');
		for (let i = 0; i < lines.length; i++) {
			const m = lines[i].match(/^(#{1,6})\s+(.*)$/);
			if (m)
				entries.push({ id: `heading-${entries.length}`, level: m[1].length, text: m[2], line: i });
		}
		return entries;
	});

	function jumpToHeading(entry: OutlineEntry) {
		if (notesState.preview) {
			document.getElementById(entry.id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
			return;
		}
		// edit mode: move the caret to the heading line and center it
		const textarea = editorEl;
		if (!textarea) return;
		const lines = notesState.content.split('\n');
		const charIndex = lines.slice(0, entry.line).reduce((sum, l) => sum + l.length + 1, 0);
		textarea.focus();
		textarea.setSelectionRange(charIndex, charIndex);
		textarea.scrollTop = Math.max(0, entry.line * 21 - textarea.clientHeight / 2);
	}

	// ---- backlinks + unlinked mentions for the open note ----
	let backlinks = $state<Backlink[]>([]);
	let unlinkedMentions = $state<Backlink[]>([]);
	$effect(() => {
		const path = notesState.selectedPath;
		// content read keeps the scan debounced while typing
		void notesState.content;
		const folder = notesState.folder;
		if (!path || !folder) {
			backlinks = [];
			unlinkedMentions = [];
			return;
		}
		const timer = setTimeout(async () => {
			backlinks = await getBacklinks(folder, path);
			unlinkedMentions = await getUnlinkedMentions(folder, path);
		}, 600);
		return () => clearTimeout(timer);
	});

	// ---- [[wiki]] links and #tag clicks in the preview ----
	// resolve a wiki name against existing notes; missing notes are created,
	// matching obsidian's behaviour
	async function openWiki(name: string) {
		const clean = name.trim();
		if (!clean) return;
		const fileName = wikiToFileName(clean);
		const match = notesState.allNotes.find((n) => n.name.toLowerCase() === fileName.toLowerCase());
		if (match) {
			await notesState.openNote(match.path);
			return;
		}
		// also try a plain name match without the .md suffix
		const loose = notesState.allNotes.find((n) =>
			n.name.toLowerCase().startsWith(clean.toLowerCase())
		);
		if (loose) {
			await notesState.openNote(loose.path);
			return;
		}
		await notesState.createNoteIn(null, clean);
	}

	// preview images resolve relative .tack/assets paths through the asset
	// protocol on desktop, through the live server in the browser
	function resolveAsset(rel: string): string {
		const folder = notesState.folder;
		if (!folder) return rel;
		const abs = rel.startsWith('/') ? rel : `${folder.replace(/\/+$/, '')}/${rel}`;
		if (!isTauri()) return noteAssetUrl(abs);
		return convertFileSrc(abs);
	}

	// convert a note into a task: title becomes the task title, the markdown
	// body the description; the note itself is left untouched
	async function convertNoteToTask(path: string) {
		const content = await notesInvoke<string>('read_file', { path }).catch(() => null);
		if (content === null) return;
		const title = path.split('/').pop()?.replace(/\.md$/i, '') ?? 'Note';
		await createTaskRepo({ title, description: content });
		window.dispatchEvent(new Event('tasks-changed'));
	}

	// ---- image attachments: paste or drop straight into the editor ----
	async function insertImageFile(file: File) {
		const bytes = new Uint8Array(await file.arrayBuffer());
		const rel = await notesState.saveAttachment(file.name || 'image.png', bytes);
		if (!rel) {
			notesState.error = 'Image could not be saved';
			return;
		}
		insertAtCaret(`![${file.name}](${rel})`);
	}

	function insertAtCaret(text: string) {
		const textarea = editorEl;
		if (!textarea) {
			notesState.content += text;
			return;
		}
		const start = textarea.selectionStart ?? notesState.content.length;
		const end = textarea.selectionEnd ?? start;
		const before = notesState.content.slice(0, start);
		const after = notesState.content.slice(end);
		notesState.content = `${before}${text}${after}`;
		const caret = start + text.length;
		tick().then(() => {
			textarea.focus();
			textarea.setSelectionRange(caret, caret);
		});
	}

	function handlePaste(e: ClipboardEvent) {
		const item = [...(e.clipboardData?.items ?? [])].find((i) => i.type.startsWith('image/'));
		if (!item) return;
		e.preventDefault();
		const file = item.getAsFile();
		if (file) void insertImageFile(file);
	}

	function handleDropImage(e: DragEvent) {
		const file = [...(e.dataTransfer?.files ?? [])].find((f) => f.type.startsWith('image/'));
		if (!file) return;
		e.preventDefault();
		void insertImageFile(file);
	}

	// ---- daily note navigation: prev/next day around a YYYY-MM-DD note ----
	const dailyDate = $derived.by(() => {
		const m = noteTitle?.match(/^(\d{4}-\d{2}-\d{2})$/);
		return m ? m[1] : null;
	});
	function navigateDaily(days: number) {
		if (!dailyDate) return;
		const next = notesState.shiftDate(dailyDate, days);
		if (next) void notesState.openDailyNote(next);
	}

	// status bar counts; reading time uses the classic 200 wpm
	const words = $derived(
		notesState.content.trim() ? notesState.content.trim().split(/\s+/).length : 0
	);
	const chars = $derived(notesState.content.length);

	// ---- find & replace (Cmd/Ctrl+F) ----
	// caret-relative like obsidian/vscode: navigation starts where you are,
	// wraps around, and esc returns to the text
	let findOpen = $state(false);
	let findQuery = $state('');
	let replaceText = $state('');
	let findIndex = $state(0);
	let findInputEl = $state<HTMLInputElement | null>(null);
	const findMatches = $derived.by(() => {
		const positions: number[] = [];
		const q = findQuery.toLowerCase();
		if (!q) return positions;
		const content = notesState.content.toLowerCase();
		let i = content.indexOf(q);
		while (i !== -1 && positions.length < 1000) {
			positions.push(i);
			i = content.indexOf(q, i + q.length);
		}
		return positions;
	});

	$effect(() => {
		void findQuery;
		if (findIndex >= findMatches.length) findIndex = 0;
	});

	// first match at/after the caret, wrapping to the top
	function nearestMatchIndex(): number {
		const caret = editorEl?.selectionStart ?? 0;
		const idx = findMatches.findIndex((p) => p >= caret);
		return idx === -1 ? 0 : idx;
	}

	// select the current match in the textarea and center its line
	function revealMatch() {
		const textarea = editorEl;
		const position = findMatches[findIndex];
		if (!textarea || position === undefined || !findQuery) return;
		textarea.focus();
		textarea.setSelectionRange(position, position + findQuery.length);
		const line = notesState.content.slice(0, position).split('\n').length - 1;
		textarea.scrollTop = Math.max(0, line * 21 - textarea.clientHeight / 2);
	}

	function openFind() {
		if (!noteTitle || notesState.preview) return;
		findOpen = true;
		void tick().then(() => {
			findIndex = nearestMatchIndex();
			findInputEl?.focus();
			findInputEl?.select();
		});
	}

	function closeFind() {
		findOpen = false;
		findQuery = '';
		replaceText = '';
	}

	function findNext(dir: 1 | -1) {
		if (findMatches.length === 0) return;
		findIndex = (findIndex + dir + findMatches.length) % findMatches.length;
		revealMatch();
	}

	// while the widget is open the editor keeps navigating: enter/shift+enter
	// move between matches, esc closes and returns to writing
	function handleFindTextareaKeydown(e: KeyboardEvent) {
		if (!findOpen) return;
		if (e.key === 'Enter') {
			e.preventDefault();
			findNext(e.shiftKey ? -1 : 1);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			closeFind();
		}
	}

	function replaceCurrent() {
		const position = findMatches[findIndex];
		if (position === undefined || !findQuery) return;
		notesState.content =
			notesState.content.slice(0, position) +
			replaceText +
			notesState.content.slice(position + findQuery.length);
		notesState.scheduleSave();
		void tick().then(() => {
			// continue right after the replaced text
			const textarea = editorEl;
			if (textarea)
				textarea.setSelectionRange(position + replaceText.length, position + replaceText.length);
			findIndex = nearestMatchIndex();
			revealMatch();
		});
	}

	function replaceAllMatches() {
		if (!findQuery) return;
		const escaped = findQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
		notesState.content = notesState.content.replace(new RegExp(escaped, 'gi'), () => replaceText);
		findIndex = 0;
		notesState.scheduleSave();
	}

	// ---- mention hover preview (notion-style peek) ----
	type MentionPreviewState = { href: string; x: number; y: number; above: boolean } | null;
	type TabDrag = { path: string };
	let mentionPreview = $state<MentionPreviewState>(null);
	let mentionPreviewTimer: ReturnType<typeof setTimeout> | undefined;
	let mentionHideTimer: ReturnType<typeof setTimeout> | undefined;

	function handleMentionHover(href: string, anchor: HTMLElement) {
		clearTimeout(mentionHideTimer);
		clearTimeout(mentionPreviewTimer);
		if (!href) return;
		// short delay so quick cursor sweeps don't flash cards
		mentionPreviewTimer = setTimeout(() => {
			const rect = anchor.getBoundingClientRect();
			const width = 320;
			const spaceBelow = window.innerHeight - rect.bottom;
			const x = Math.min(Math.max(8, rect.left), window.innerWidth - width - 12);
			const above = spaceBelow < 240;
			mentionPreview = {
				href,
				x,
				y: above ? rect.top - 8 : rect.bottom + 8,
				above
			};
		}, 350);
	}

	function handleMentionLeave() {
		clearTimeout(mentionPreviewTimer);
		// grace period so the pointer can move onto the card itself
		mentionHideTimer = setTimeout(() => (mentionPreview = null), 150);
	}

	function keepMentionPreview() {
		clearTimeout(mentionHideTimer);
	}

	function dismissMentionPreview() {
		mentionPreview = null;
		clearTimeout(mentionPreviewTimer);
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

{#snippet outlinePanel()}
	<aside class="w-52 shrink-0 overflow-y-auto border-l border-border/60 pl-3">
		<p class="pb-2 text-[11px] font-medium text-muted-foreground/70">Outline</p>
		{#if outline.length === 0}
			<p class="text-[11px] text-muted-foreground/50">No headings yet</p>
		{:else}
			{#each outline as entry (entry.id)}
				<button
					type="button"
					class="block w-full truncate rounded py-0.5 text-left text-[12px] text-muted-foreground transition-colors hover:text-foreground"
					style="padding-left: {(entry.level - 1) * 10}px"
					onclick={() => jumpToHeading(entry)}
				>
					{entry.text}
				</button>
			{/each}
		{/if}
	</aside>
{/snippet}

<section class="flex h-full flex-col px-3 py-3 sm:px-5 sm:py-5 lg:px-8 lg:py-8">
	{#if notesState.noteTabs.length > 0 && !notesState.focusMode}
		<!-- editor tabs: vs code/chrome pattern softened to match the app chrome -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="-mx-3 -mt-3 mb-3 flex [scrollbar-width:none] items-end gap-0.5 overflow-x-auto border-b border-border px-3 pt-1 sm:-mx-5 sm:-mt-5 sm:px-5 lg:-mx-8 lg:px-8 [&::-webkit-scrollbar]:hidden"
			ondblclick={() => void notesState.createNote()}
		>
			{#each notesState.noteTabs as tabPath (tabPath)}
				{@const tabName = tabPath.split('/').pop()?.replace(/\.md$/, '') ?? tabPath}
				<ContextMenu.Root>
					<ContextMenu.Trigger class="contents">
						<div
							role="listitem"
							class="group flex h-8 max-w-[11rem] shrink-0 items-center gap-1.5 rounded-t-md border border-b-0 px-2.5 text-[12px] transition-colors {tabPath ===
							notesState.selectedPath
								? 'border-border bg-muted/40 text-foreground'
								: 'border-transparent text-muted-foreground hover:bg-muted/20 hover:text-foreground'}"
							use:sortableItem={{
								dragData: { path: tabPath },
								container: 'note-tabs',
								direction: 'horizontal',
								onDrop: (s: DragDropState<TabDrag>) => {
									const dragged = s.draggedItem;
									if (!dragged || !s.dropPosition) return;
									notesState.reorderTabs(dragged.path, tabPath, s.dropPosition);
								}
							}}
						>
							<span
								role="button"
								tabindex="0"
								class="min-w-0 cursor-pointer text-left"
								onclick={() => void notesState.openNote(tabPath)}
								onkeydown={(e) => {
									if (e.key === 'Enter' || e.key === ' ') {
										e.preventDefault();
										void notesState.openNote(tabPath);
									}
								}}
								onauxclick={(e) => {
									if (e.button === 1) notesState.closeTab(tabPath);
								}}
							>
								<span class="block truncate">{tabName}</span>
							</span>
							<span
								role="button"
								tabindex="-1"
								aria-label="Close tab"
								class="flex h-4 w-4 shrink-0 cursor-pointer items-center justify-center rounded text-muted-foreground/70 transition-opacity hover:bg-muted hover:text-foreground {tabPath ===
								notesState.selectedPath
									? 'opacity-100'
									: 'opacity-0 group-hover:opacity-100'}"
								onclick={(e) => {
									e.stopPropagation();
									notesState.closeTab(tabPath);
								}}
								onkeydown={(e) => {
									if (e.key === 'Enter' || e.key === ' ') {
										e.preventDefault();
										notesState.closeTab(tabPath);
									}
								}}
							>
								<svg class="size-3" viewBox="0 0 24 24" fill="none" aria-hidden="true">
									<path
										fill="currentColor"
										d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
									/>
								</svg>
							</span>
						</div>
					</ContextMenu.Trigger>
					<ContextMenu.Content class="w-44">
						<ContextMenu.Item onclick={() => notesState.closeTab(tabPath)}>
							Close tab
						</ContextMenu.Item>
						<ContextMenu.Item onclick={() => notesState.closeOtherTabs(tabPath)}>
							Close other tabs
						</ContextMenu.Item>
						<ContextMenu.Item
							onclick={() => notesState.closeAllTabs()}
							class="text-destructive data-[highlighted]:text-destructive"
						>
							Close all tabs
						</ContextMenu.Item>
					</ContextMenu.Content>
				</ContextMenu.Root>
			{/each}
		</div>
	{/if}
	<header class="flex flex-wrap items-center justify-between gap-2 pb-4 sm:pb-5">
		<div class="flex min-w-0 flex-1 items-center gap-3">
			{#if dailyDate}
				<!-- daily note navigation: prev / today / next like obsidian's daily notes -->
				<div class="flex items-center gap-0.5">
					<Button
						variant="ghost"
						size="icon-sm"
						class="size-7 text-muted-foreground hover:text-foreground"
						aria-label="Previous day"
						onclick={() => navigateDaily(-1)}
					>
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M7.058 15.944a1 1 0 0 1-1.28.095l-.094-.08-.057-.058-4.575-4.575a1 1 0 0 1 0-1.431l4.575-4.576a1 1 0 0 1 1.431 1.431L3.911 10.5h16.087a1 1 0 0 1 0 2H3.91l3.148 3.148a1 1 0 0 1 0 1.431z"
							/></svg
						>
					</Button>
					<Button
						variant="ghost"
						size="sm"
						class="h-7 text-[12px] text-muted-foreground hover:text-foreground"
						onclick={() => void notesState.openDailyNote(new Date().toLocaleDateString('en-CA'))}
					>
						Today
					</Button>
					<Button
						variant="ghost"
						size="icon-sm"
						class="size-7 text-muted-foreground hover:text-foreground"
						aria-label="Next day"
						onclick={() => navigateDaily(1)}
					>
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M16.942 15.944a1 1 0 0 0 1.28.095l.094-.08.057-.058 4.575-4.575a1 1 0 0 0 0-1.431l-4.575-4.576a1 1 0 0 0-1.431 1.431l3.147 3.15H4.002a1 1 0 0 0 0 2h16.087l-3.148 3.148a1 1 0 0 0 0 1.431z"
							/></svg
						>
					</Button>
				</div>
			{/if}
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
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon-sm"
								class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {outlineOpen
									? 'bg-muted text-foreground'
									: 'text-muted-foreground hover:text-foreground'}"
								onclick={() => (outlineOpen = !outlineOpen)}
								aria-label="Toggle outline"
							>
								<svg class="size-[15px]" viewBox="0 0 24 24" fill="none" aria-hidden="true"
									><path
										fill="currentColor"
										d="M7 13a2 2 0 0 1 1.995 1.85L9 15v3a2 2 0 0 1-1.85 1.995L7 20H4a2 2 0 0 1-1.995-1.85L2 18v-3a2 2 0 0 1 1.85-1.995L4 13zm9 4a1 1 0 0 1 .117 1.993L16 19h-4a1 1 0 0 1-.117-1.993L12 17zm4-4a1 1 0 1 1 0 2h-8a1 1 0 1 1 0-2zM7 3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zm9 4a1 1 0 0 1 .117 1.993L16 9h-4a1 1 0 0 1-.117-1.993L12 7zm4-4a1 1 0 0 1 .117 1.993L20 5h-8a1 1 0 0 1-.117-1.993L12 3z"
									/></svg
								>
							</Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="bottom">Outline</Tooltip.Content>
				</Tooltip.Root>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="icon-sm"
								class="flex h-7 w-7 items-center justify-center rounded-md transition-colors {notesState.focusMode
									? 'bg-muted text-foreground'
									: 'text-muted-foreground hover:text-foreground'}"
								onclick={() => (notesState.focusMode = !notesState.focusMode)}
								aria-label="Toggle focus mode"
							>
								<svg width="15" height="15" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M12 2a1 1 0 0 1 .993.883L13 3v.055a9.005 9.005 0 0 1 7.911 7.674l.034.271H21a1 1 0 0 1 .117 1.993L21 13h-.055a9.005 9.005 0 0 1-7.674 7.911l-.271.034V21a1 1 0 0 1-1.993.117L11 21v-.055a9.005 9.005 0 0 1-7.911-7.674L3.055 13H3a1 1 0 0 1-.117-1.993L3 11h.055a9.005 9.005 0 0 1 7.674-7.911L11 3.055V3a1 1 0 0 1 1-1m0 5a5 5 0 1 0 0 10 5 5 0 0 0 0-10m0 2a3 3 0 1 1 0 6 3 3 0 0 1 0-6"
									/></svg
								>
							</Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="bottom">Focus mode</Tooltip.Content>
				</Tooltip.Root>
			</div>
		{/if}
	</header>
	{#if noteTitle && (backlinks.length > 0 || unlinkedMentions.length > 0) && !notesState.focusMode}
		<!-- backlinks: notes that link here via @-mentions; unlinked mentions
		     contain the note title as plain text -->
		<div class="flex flex-wrap items-center gap-1.5 pb-3">
			{#if backlinks.length > 0}
				<span class="text-[11px] text-muted-foreground/70">Linked notes</span>
				{#each backlinks as backlink (backlink.path)}
					<button
						type="button"
						class="max-w-52 truncate rounded-full border border-border bg-muted/30 px-2 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
						onclick={() => void notesState.openNote(backlink.path)}
					>
						{backlink.name.replace(/\.md$/, '')}
					</button>
				{/each}
			{/if}
			{#if unlinkedMentions.length > 0}
				<span class="text-[11px] text-muted-foreground/70">Unlinked mentions</span>
				{#each unlinkedMentions as backlink (backlink.path)}
					<button
						type="button"
						class="max-w-52 truncate rounded-full border border-dashed border-border bg-transparent px-2 py-0.5 text-[11px] text-muted-foreground/80 transition-colors hover:bg-muted/60 hover:text-foreground"
						onclick={() => void notesState.openNote(backlink.path)}
					>
						{backlink.name.replace(/\.md$/, '')}
					</button>
				{/each}
			{/if}
		</div>
	{/if}
	{#if !notesState.folder}
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
		<div class="flex min-h-0 flex-1 gap-4">
			<div class="min-h-0 flex-1 overflow-auto pl-1">
				<MarkdownRenderer
					content={notesState.content}
					onToggleLine={toggleTodoLine}
					onOpenMention={(href) => void openMention(href)}
					onMentionHover={handleMentionHover}
					onMentionLeave={handleMentionLeave}
					onOpenWiki={(name) => void openWiki(name)}
					onOpenTag={(tag) => notesState.setActiveTag(tag)}
					{resolveAsset}
				/>
			</div>
			{#if outlineOpen}
				{@render outlinePanel()}
			{/if}
		</div>
	{:else}
		<!-- editor; typing "/" at line start opens the notion-style block menu,
		     right-click on text opens the selection effects dropdown -->
		<div class="flex min-h-0 flex-1 gap-4">
			<div class="relative min-h-0 flex-1">
				<textarea
					bind:this={editorEl}
					bind:value={notesState.content}
					oninput={handleEditorInput}
					oncontextmenu={handleEditorContextMenu}
					onkeydown={handleFindTextareaKeydown}
					onpaste={handlePaste}
					ondrop={handleDropImage}
					spellcheck={notesState.spellcheck}
					placeholder="Write in markdown…"
					class="h-full w-full resize-none bg-transparent p-3 font-mono text-[13px] leading-[21px] text-foreground outline-none placeholder:text-muted-foreground/50"
				></textarea>
				{#if findOpen}
					<!-- find & replace widget, vs-code style top-right overlay -->
					<div
						class="absolute top-2 right-3 z-20 w-76 rounded-lg border border-border bg-popover p-2 shadow-lg"
					>
						<div class="flex items-center gap-1.5">
							<Input
								bind:ref={findInputEl}
								bind:value={findQuery}
								placeholder="Find"
								spellcheck="false"
								class="h-7 flex-1 text-[12px]"
								onkeydown={(e) => {
									if (e.key === 'Enter') {
										e.preventDefault();
										findNext(e.shiftKey ? -1 : 1);
									} else if (e.key === 'Escape') {
										closeFind();
									}
								}}
							/>
							<span class="shrink-0 text-[11px] text-muted-foreground tabular-nums">
								{findMatches.length ? `${findIndex + 1}/${findMatches.length}` : '0/0'}
							</span>
							<Button
								variant="ghost"
								size="icon-sm"
								class="size-7 text-muted-foreground hover:text-foreground"
								aria-label="Previous match"
								onclick={() => findNext(-1)}
							>
								<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M7.058 15.944a1 1 0 0 1-1.28.095l-.094-.08-.057-.058-4.575-4.575a1 1 0 0 1 0-1.431l4.575-4.576a1 1 0 0 1 1.431 1.431L3.911 10.5h16.087a1 1 0 0 1 0 2H3.91l3.148 3.148a1 1 0 0 1 0 1.431z"
									/></svg
								>
							</Button>
							<Button
								variant="ghost"
								size="icon-sm"
								class="size-7 text-muted-foreground hover:text-foreground"
								aria-label="Next match"
								onclick={() => findNext(1)}
							>
								<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M16.942 15.944a1 1 0 0 0 1.28.095l.094-.08.057-.058 4.575-4.575a1 1 0 0 0 0-1.431l-4.575-4.576a1 1 0 0 0-1.431 1.431l3.147 3.15H4.002a1 1 0 0 0 0 2h16.087l-3.148 3.148a1 1 0 0 0 0 1.431z"
									/></svg
								>
							</Button>
							<Button
								variant="ghost"
								size="icon-sm"
								class="size-7 text-muted-foreground hover:text-foreground"
								aria-label="Close find"
								onclick={closeFind}
							>
								<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
									><path
										fill="currentColor"
										d="M18.3 5.71a1 1 0 0 0-1.42 0L12 10.59l-4.88-4.88a1 1 0 1 0-1.42 1.42L10.59 12l-4.88 4.88a1 1 0 1 0 1.41 1.42L12 13.41l4.88 4.88a1 1 0 0 0 1.42-1.42L13.41 12l4.88-4.88a1 1 0 0 0 0-1.41Z"
									/></svg
								>
							</Button>
						</div>
						<div class="mt-1.5 flex items-center gap-1.5">
							<Input
								bind:value={replaceText}
								placeholder="Replace"
								spellcheck="false"
								class="h-7 min-w-0 flex-1 text-[12px]"
							/>
							<Button variant="outline" size="sm" class="h-7 text-[12px]" onclick={replaceCurrent}
								>Replace</Button
							>
							<Button
								variant="outline"
								size="sm"
								class="h-7 text-[12px]"
								onclick={replaceAllMatches}>All</Button
							>
						</div>
					</div>
				{/if}
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
			{#if outlineOpen}
				{@render outlinePanel()}
			{/if}
		</div>
		{#if !notesState.focusMode}
			<!-- status bar: word/char count left, saving state + toggles right -->
			<div
				class="flex items-center justify-between border-t border-border/60 pt-2 text-[11px] text-muted-foreground tabular-nums"
			>
				<span>
					{words} words · {chars} chars
				</span>
				<span class="flex items-center gap-3">
					{#if notesState.saving}
						<span>Saving…</span>
					{/if}
					<Tooltip.Root>
						<Tooltip.Trigger>
							{#snippet child({ props })}
								<button
									{...props}
									type="button"
									class="transition-colors hover:text-foreground {notesState.spellcheck
										? 'text-foreground'
										: ''}"
									onclick={() => notesState.toggleSpellcheck()}
								>
									Spellcheck
								</button>
							{/snippet}
						</Tooltip.Trigger>
						<Tooltip.Content side="top">
							Toggle the browser's spell checker in the editor
						</Tooltip.Content>
					</Tooltip.Root>
				</span>
			</div>
		{/if}
	{/if}
	{#if mentionPreview}
		<!-- fixed peek card anchored to the hovered mention link -->
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<div
			class="fixed z-50"
			style="left: {mentionPreview.x}px; top: {mentionPreview.y}px; transform: translate(0, {mentionPreview.above
				? '-100%'
				: '0'});"
			onmouseenter={keepMentionPreview}
			onmouseleave={handleMentionLeave}
			onclick={() => {
				const href = mentionPreview?.href;
				dismissMentionPreview();
				if (href) void openMention(href);
			}}
		>
			<MentionPreviewCard href={mentionPreview.href} />
		</div>
	{/if}
</section>
