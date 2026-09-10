import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { isTauri } from '$lib/db/client';
import { reorderArray } from '$lib/dnd';
import { reindexNotes, indexNote } from './search';

export type NoteInfo = { name: string; path: string; modified: number };

const FOLDER_KEY = 'tack-notes-folder';
const TAB_KEY = 'tack-notes-tab';
const SORT_KEY = 'tack-notes-sort';
const EXPANDED_KEY = 'tack-notes-expanded';
const ORDER_KEY = 'tack-notes-order';

function loadInitialTab(): 'tasks' | 'notes' {
	try {
		return localStorage.getItem(TAB_KEY) === 'notes' ? 'notes' : 'tasks';
	} catch {
		return 'tasks';
	}
}

// shared between the sidebar tabs, the notes sidebar and the editor
class NotesPageState {
	activeTab = $state<'tasks' | 'notes'>(loadInitialTab());
	folder = $state<string | null>(null);
	notes = $state<NoteInfo[]>([]);
	// relative folder paths (nested) and their notes, grouped by folder
	folders = $state<string[]>([]);
	folderNotes = $state<Record<string, NoteInfo[]>>({});
	// which folders show their children; missing key means expanded
	expandedFolders = $state<Record<string, boolean>>({});
	archived = $state<NoteInfo[]>([]);
	pinned = $state<string[]>([]);
	sortBy = $state<'modified' | 'name' | 'manual'>('modified');
	visitCounts = $state<Record<string, number>>({});
	selectedPath = $state<string | null>(null);
	// open editor tabs (ordered note paths); only the active tab loads content
	noteTabs = $state<string[]>([]);
	content = $state('');
	preview = $state(false);
	loading = $state(false);
	saving = $state(false);
	error = $state<string | null>(null);
	// name-too-long is a user input problem: shown as a dialog, not the error banner
	nameWarning = $state<string | null>(null);

	// manual drag order per container ('root' or a relative folder path)
	#manualOrders: Record<string, string[]> = {};

	// per-tab content cache: tab switches skip the disk read; cleared on
	// refresh so external edits never show stale text
	#contentCache = new Map<string, string>();

	#initialized = false;
	#saveTimer: ReturnType<typeof setTimeout> | undefined;

	static #instance: NotesPageState | undefined;

	constructor() {
		// remember the active tab across page reloads
		$effect.root(() => {
			$effect(() => {
				try {
					localStorage.setItem(TAB_KEY, this.activeTab);
				} catch {
					// storage unavailable (ssr) - not worth reporting
				}
			});
			// remember which folders are collapsed
			$effect(() => {
				try {
					localStorage.setItem(EXPANDED_KEY, JSON.stringify(this.expandedFolders));
				} catch {
					// storage unavailable (ssr) - not worth reporting
				}
			});
		});
	}

	static get(): NotesPageState {
		this.#instance ??= new NotesPageState();
		return this.#instance;
	}

	async init() {
		if (this.#initialized || !isTauri()) return;
		this.#initialized = true;
		this.folder = localStorage.getItem(FOLDER_KEY);
		const storedSort = localStorage.getItem(SORT_KEY);
		this.sortBy = storedSort === 'name' || storedSort === 'manual' ? storedSort : 'modified';
		try {
			this.expandedFolders = JSON.parse(localStorage.getItem(EXPANDED_KEY) ?? '{}') ?? {};
		} catch {
			this.expandedFolders = {};
		}
		if (this.folder) await this.refresh();
		await this.#loadTabs();
	}

	async refresh() {
		if (!this.folder) return;
		this.loading = true;
		this.error = null;
		// disk state changed elsewhere; cached tab contents are no longer trusted
		this.#contentCache.clear();
		try {
			// one recursive listing; notes are grouped by their parent folder
			const deep = await invoke<NoteInfo[]>('list_notes_deep', { dir: this.folder });
			const rootAbs = this.folder.replace(/\/+$/, '');
			const root: NoteInfo[] = [];
			const byFolder: Record<string, NoteInfo[]> = {};
			for (const note of deep) {
				const rel = note.path.slice(rootAbs.length + 1).replace(/\\/g, '/');
				const parent = rel.includes('/') ? rel.slice(0, rel.lastIndexOf('/')) : '';
				if (parent) (byFolder[parent] ??= []).push(note);
				else root.push(note);
			}
			this.notes = root;
			this.folderNotes = byFolder;
			this.folders = await invoke<string[]>('list_note_folders', { dir: this.folder });
			// missing archive folder just means nothing is archived yet
			this.archived = await invoke<NoteInfo[]>('list_notes', { dir: this.archiveDir }).catch(
				() => []
			);
			this.#loadPins();
			this.#loadVisits();
			this.#loadOrders();
			// drop tabs for notes that no longer exist on disk
			const known = new Set(this.allNotes.map((n) => n.path));
			if (this.noteTabs.some((t) => !known.has(t))) {
				this.noteTabs = this.noteTabs.filter((t) => known.has(t));
				this.#persistTabs();
			}
			// rebuild the search index so external edits stay searchable; best effort
			try {
				const full = await invoke<{ path: string; name: string; content: string }[]>(
					'read_notes_deep',
					{ dir: this.folder }
				);
				await reindexNotes(full);
			} catch {
				// search stays stale rather than breaking the notes list
			}
		} catch (e) {
			this.error = String(e);
			this.notes = [];
			this.folders = [];
			this.folderNotes = {};
			this.archived = [];
		} finally {
			this.loading = false;
		}
	}

	#loadPins() {
		if (!this.folder) return;
		try {
			const stored = JSON.parse(localStorage.getItem(`tack-notes-pinned:${this.folder}`) ?? '[]');
			this.pinned = Array.isArray(stored)
				? stored.filter((n: string) => this.allNotes.some((note) => note.name === n))
				: [];
		} catch {
			this.pinned = [];
		}
	}

	togglePin(name: string) {
		if (!this.folder) return;
		this.pinned = this.pinned.includes(name)
			? this.pinned.filter((n) => n !== name)
			: [...this.pinned, name];
		localStorage.setItem(`tack-notes-pinned:${this.folder}`, JSON.stringify(this.pinned));
	}

	#loadVisits() {
		if (!this.folder) return;
		try {
			const stored = JSON.parse(localStorage.getItem(`tack-notes-visits:${this.folder}`) ?? '{}');
			const counts: Record<string, number> = stored && typeof stored === 'object' ? stored : {};
			// drop entries for notes that no longer exist anywhere
			const names = new Set([...this.allNotes, ...this.archived].map((n) => n.name));
			this.visitCounts = Object.fromEntries(Object.entries(counts).filter(([k]) => names.has(k)));
		} catch {
			this.visitCounts = {};
		}
	}

	noteOpened(name: string) {
		if (!this.folder) return;
		this.visitCounts = { ...this.visitCounts, [name]: (this.visitCounts[name] ?? 0) + 1 };
		localStorage.setItem(`tack-notes-visits:${this.folder}`, JSON.stringify(this.visitCounts));
	}

	// most visited notes, at most four; feeds the empty-state grid
	get recentNotes(): NoteInfo[] {
		return [...this.allNotes]
			.filter((n) => (this.visitCounts[n.name] ?? 0) > 0)
			.sort((a, b) => (this.visitCounts[b.name] ?? 0) - (this.visitCounts[a.name] ?? 0))
			.slice(0, 4);
	}

	// every note in the tree, including subfolder notes
	get allNotes(): NoteInfo[] {
		return [...this.notes, ...Object.values(this.folderNotes).flat()];
	}

	setSort(sort: 'modified' | 'name' | 'manual') {
		this.sortBy = sort;
		localStorage.setItem(SORT_KEY, sort);
	}

	// notes of one container in display order: pinned first, then the chosen sort
	sortedNotesIn(notes: NoteInfo[], container: string): NoteInfo[] {
		const isPinned = (n: NoteInfo) => this.pinned.includes(n.name);
		const pinned = notes.filter(isPinned);
		const rest = notes.filter((n) => !isPinned(n));
		if (this.sortBy === 'manual') {
			const order = this.#manualOrders[container] ?? [];
			const rank = (n: NoteInfo) => {
				const i = order.indexOf(n.name);
				return i === -1 ? order.length : i;
			};
			rest.sort((a, b) => rank(a) - rank(b));
		} else {
			rest.sort((a, b) =>
				this.sortBy === 'name' ? a.name.localeCompare(b.name) : b.modified - a.modified
			);
		}
		return [...pinned, ...rest];
	}

	// pinned notes first, then by the chosen sort order
	get sortedNotes(): NoteInfo[] {
		return this.sortedNotesIn(this.notes, 'root');
	}

	// display order for a folder's notes
	sortedFolderNotes(rel: string): NoteInfo[] {
		return this.sortedNotesIn(this.folderNotes[rel] ?? [], rel);
	}

	// persist a drag reorder; switches the sort to manual so the order sticks
	applyOrder(container: string, ordered: NoteInfo[]) {
		this.#manualOrders[container] = ordered.map((n) => n.name);
		try {
			localStorage.setItem(`${ORDER_KEY}:${this.folder}`, JSON.stringify(this.#manualOrders));
		} catch {
			// storage unavailable - order is lost on reload, not critical
		}
		if (this.sortBy !== 'manual') this.setSort('manual');
	}

	#loadOrders() {
		if (!this.folder) return;
		try {
			const stored = JSON.parse(localStorage.getItem(`${ORDER_KEY}:${this.folder}`) ?? '{}');
			this.#manualOrders = stored && typeof stored === 'object' ? stored : {};
		} catch {
			this.#manualOrders = {};
		}
	}

	toggleFolder(rel: string) {
		const next = !(this.expandedFolders[rel] ?? true);
		this.expandedFolders = { ...this.expandedFolders, [rel]: next };
	}

	isExpanded(rel: string): boolean {
		return this.expandedFolders[rel] ?? true;
	}

	get archiveDir(): string {
		return this.folder ? `${this.folder.replace(/\/+$/, '')}/.tack/archive` : '';
	}

	get trashDir(): string {
		return this.folder ? `${this.folder.replace(/\/+$/, '')}/.tack/trash` : '';
	}

	async pickFolder() {
		const selection = await openDialog({
			directory: true,
			multiple: false,
			title: 'Choose notes folder'
		});
		if (typeof selection !== 'string') return;
		await this.setFolder(selection);
	}

	async setFolder(dir: string) {
		this.flushPendingSave();
		this.folder = dir;
		localStorage.setItem(FOLDER_KEY, dir);
		this.selectedPath = null;
		this.content = '';
		this.noteTabs = [];
		await this.refresh();
		await this.#loadTabs();
	}

	async openNote(path: string, countVisit = true) {
		if (path === this.selectedPath) return;
		this.flushPendingSave();
		this.selectedPath = path;
		this.preview = false;
		this.content = '';
		const name = path.split('/').pop();
		if (name && countVisit) this.noteOpened(name);
		// cached content makes switching back to a tab instant
		const cached = this.#contentCache.get(path);
		if (cached !== undefined) {
			this.content = cached;
			if (!this.noteTabs.includes(path)) this.noteTabs = [...this.noteTabs, path];
			this.#persistTabs();
			return;
		}
		try {
			this.content = await invoke<string>('read_file', { path });
			this.#contentCache.set(path, this.content);
			// the tab is only kept once the read succeeds, so dead links never
			// leave an empty tab behind
			if (!this.noteTabs.includes(path)) this.noteTabs = [...this.noteTabs, path];
			this.#persistTabs();
		} catch {
			this.noteTabs = this.noteTabs.filter((t) => t !== path);
			this.#persistTabs();
			// drop the selection first so a pending save can't recreate the file as an empty shell
			this.selectedPath = null;
			this.content = '';
			// stale mention link: heal it by finding the note with the same file name
			const match = name
				? [...this.allNotes, ...this.archived].find((n) => n.name === name && n.path !== path)
				: undefined;
			if (match) {
				await this.#rewriteMentionLinks(new Map([[path, match.path]]));
				await this.openNote(match.path, countVisit);
				return;
			}
			this.error = 'Note not found — the link may be outdated (note moved or deleted)';
		}
	}

	closeTab(path: string) {
		const idx = this.noteTabs.indexOf(path);
		if (idx === -1) return;
		this.noteTabs = this.noteTabs.filter((t) => t !== path);
		this.#persistTabs();
		if (path !== this.selectedPath) return;
		this.selectedPath = null;
		this.content = '';
		// activate the right neighbor, else the left one (vs code behaviour)
		const next = this.noteTabs[idx] ?? this.noteTabs[idx - 1] ?? null;
		if (next) void this.openNote(next);
	}

	cycleTab(delta: 1 | -1) {
		if (this.noteTabs.length < 2 || !this.selectedPath) return;
		const idx = this.noteTabs.indexOf(this.selectedPath);
		const next = this.noteTabs[(idx + delta + this.noteTabs.length) % this.noteTabs.length];
		if (next) void this.openNote(next);
	}

	// drag-reorder of the tab strip
	reorderTabs(dragged: string, target: string, dropPosition: 'before' | 'after') {
		if (dragged === target) return;
		this.noteTabs = reorderArray([...this.noteTabs], dragged, target, dropPosition);
		this.#persistTabs();
	}

	closeOtherTabs(path: string) {
		if (!this.noteTabs.includes(path)) return;
		this.noteTabs = [path];
		this.#persistTabs();
		if (this.selectedPath !== path) void this.openNote(path);
	}

	closeAllTabs() {
		this.noteTabs = [];
		this.#persistTabs();
		this.selectedPath = null;
		this.content = '';
	}

	// tabs + active note survive restarts; only paths are stored, content loads on activation
	#persistTabs() {
		if (!this.folder) return;
		try {
			localStorage.setItem(
				`tack-notes-tabs:${this.folder}`,
				JSON.stringify({ paths: this.noteTabs, active: this.selectedPath })
			);
		} catch {
			// storage unavailable (ssr) - not worth reporting
		}
	}

	async #loadTabs() {
		if (!this.folder || !isTauri()) return;
		let paths: string[] = [];
		let active: string | null = null;
		try {
			const stored = JSON.parse(localStorage.getItem(`tack-notes-tabs:${this.folder}`) ?? '{}');
			if (Array.isArray(stored?.paths)) {
				paths = stored.paths.filter((p: unknown) => typeof p === 'string');
			}
			if (typeof stored?.active === 'string') active = stored.active;
		} catch {
			// no stored tabs is fine
		}
		const known = new Set(this.allNotes.map((n) => n.path));
		this.noteTabs = paths.filter((p) => known.has(p));
		const target =
			active && this.noteTabs.includes(active) ? active : (this.noteTabs.at(-1) ?? null);
		if (target) await this.openNote(target, false);
	}

	// open (or create) the daily note named YYYY-MM-DD.md in the notes root
	async openTodayNote() {
		if (!this.folder) return;
		const today = new Date().toLocaleDateString('en-CA');
		const name = `${today}.md`;
		const existing = this.allNotes.find((n) => n.name.toLowerCase() === name.toLowerCase());
		if (existing) await this.openNote(existing.path);
		else await this.createNoteIn(null, today);
	}

	scheduleSave() {
		if (!this.selectedPath) return;
		clearTimeout(this.#saveTimer);
		this.#saveTimer = setTimeout(() => void this.#save(), 500);
	}

	flushPendingSave() {
		if (this.#saveTimer === undefined) return;
		clearTimeout(this.#saveTimer);
		this.#saveTimer = undefined;
		void this.#save();
	}

	async #save() {
		if (!this.selectedPath) return;
		this.saving = true;
		try {
			await invoke('write_file', { path: this.selectedPath, content: this.content });
			this.#contentCache.set(this.selectedPath, this.content);
			const name = this.selectedPath.split('/').pop() ?? '';
			await indexNote(this.selectedPath, name, this.content).catch(() => {});
		} catch (e) {
			this.error = String(e);
		} finally {
			this.saving = false;
		}
	}

	// mention links embed absolute file paths; after a rename/move every
	// @[label](note:OLD) reference must be rewritten to the new path
	async #rewriteMentionLinks(remap: Map<string, string>) {
		if (!this.folder || remap.size === 0) return;
		this.flushPendingSave();
		const all = await invoke<{ path: string; content: string }[]>('read_notes_deep', {
			dir: this.folder
		}).catch(() => []);
		for (const note of all) {
			let content = note.content;
			let changed = false;
			for (const [oldPath, newPath] of remap) {
				const token = `](note:${encodeURIComponent(oldPath)})`;
				if (content.includes(token)) {
					content = content.split(token).join(`](note:${encodeURIComponent(newPath)})`);
					changed = true;
				}
			}
			if (!changed) continue;
			await invoke('write_file', { path: note.path, content });
			// keep the open editor in sync if its content was just rewritten
			if (note.path === this.selectedPath) this.content = content;
		}
		// keep editor tabs pointing at the renamed/moved paths
		if (this.noteTabs.some((t) => remap.has(t))) {
			this.noteTabs = this.noteTabs.map((t) => remap.get(t) ?? t);
			if (this.selectedPath && remap.has(this.selectedPath)) {
				this.selectedPath = remap.get(this.selectedPath) ?? this.selectedPath;
			}
			// keep the content cache consistent with the new paths
			for (const [oldPath, newPath] of remap) {
				const content = this.#contentCache.get(oldPath);
				if (content !== undefined) {
					this.#contentCache.delete(oldPath);
					this.#contentCache.set(newPath, content);
				}
			}
			this.#persistTabs();
		}
	}

	// remap for every note inside a moved/renamed folder
	#folderRemap(rel: string, nextRel: string): Map<string, string> {
		const remap = new Map<string, string>();
		if (!this.folder) return remap;
		const rootAbs = this.folder.replace(/\/+$/, '');
		for (const note of this.allNotes) {
			if (note.path.startsWith(`${rootAbs}/${rel}/`)) {
				remap.set(
					note.path,
					`${rootAbs}/${nextRel}/${note.path.slice(rootAbs.length + rel.length + 2)}`
				);
			}
		}
		return remap;
	}

	// sanitize a user-typed name into a safe file/folder name
	#sanitizeName(raw: string): string | null {
		const name = raw
			.trim()
			.replace(/[/\\:]/g, '-')
			.replace(/\.md$/i, '');
		return name && name !== '.' && name !== '..' ? name : null;
	}

	// route fs failures: name-too-long warns in a dialog, everything else
	// goes to the error banner
	#fail(e: unknown) {
		const msg = String(e);
		if (/os error 63|file name too long/i.test(msg)) {
			this.nameWarning = 'This name is too long for the file system. Please pick a shorter one.';
			return;
		}
		this.error = msg;
	}

	async createNote() {
		await this.createNoteIn(null);
	}

	// create a note in the given folder (null = notes root); empty name becomes Untitled
	async createNoteIn(parentRel: string | null, rawName?: string) {
		if (!this.folder) return;
		this.flushPendingSave();
		const rootAbs = this.folder.replace(/\/+$/, '');
		const base = this.#sanitizeName(rawName ?? '') ?? 'Untitled';
		const siblings = parentRel ? (this.folderNotes[parentRel] ?? []) : this.notes;
		// first name that does not exist yet: Untitled.md, Untitled 2.md, ...
		let name = `${base}.md`;
		for (let i = 2; siblings.some((n) => n.name === name); i++) name = `${base} ${i}.md`;
		const path = parentRel ? `${rootAbs}/${parentRel}/${name}` : `${rootAbs}/${name}`;
		try {
			await invoke('write_file', { path, content: '' });
		} catch (e) {
			this.#fail(e);
			return;
		}
		await this.refresh();
		// creating a note is not a visit: "Jump back in" counts only real opens
		await this.openNote(path, false);
	}

	// create a folder under the given parent (null = notes root)
	async createFolder(parentRel: string | null, rawName: string) {
		if (!this.folder) return;
		const name = this.#sanitizeName(rawName);
		if (!name) return;
		// relative target so nesting works: parent/name
		const target = parentRel ? `${parentRel}/${name}` : name;
		try {
			await invoke('create_folder', { dir: this.folder, name: target });
		} catch (e) {
			this.#fail(e);
			return;
		}
		await this.refresh();
	}

	// move a note file into a folder (null = notes root)
	async moveNote(path: string, parentRel: string | null) {
		if (!this.folder) return;
		const name = path.split('/').pop();
		if (!name) return;
		const rootAbs = this.folder.replace(/\/+$/, '');
		const newPath = parentRel ? `${rootAbs}/${parentRel}/${name}` : `${rootAbs}/${name}`;
		if (newPath === path) return;
		if (this.allNotes.some((n) => n.path === newPath)) {
			this.error = 'A note with this name already exists';
			return;
		}
		try {
			await invoke('rename_note', { oldPath: path, newPath });
		} catch (e) {
			this.#fail(e);
			return;
		}
		// the note left its container; drop it from every manual order list
		for (const key of Object.keys(this.#manualOrders)) {
			this.#manualOrders[key] = this.#manualOrders[key].filter((n) => n !== name);
		}
		if (this.selectedPath === path) this.selectedPath = newPath;
		await this.#rewriteMentionLinks(new Map([[path, newPath]]));
		await this.refresh();
	}

	// move a folder into another folder; refuses to move into itself/descendants
	async moveFolder(rel: string, parentRel: string | null) {
		if (!this.folder || parentRel === rel) return;
		if (parentRel && (parentRel.startsWith(`${rel}/`) || parentRel === rel)) return;
		const name = rel.split('/').pop() ?? '';
		if (!name) return;
		const rootAbs = this.folder.replace(/\/+$/, '');
		const nextRel = parentRel ? `${parentRel}/${name}` : name;
		if (nextRel === rel) return;
		if (this.folders.includes(nextRel)) {
			this.error = 'A folder with this name already exists';
			return;
		}
		try {
			await invoke('rename_note', {
				oldPath: `${rootAbs}/${rel}`,
				newPath: `${rootAbs}/${nextRel}`
			});
		} catch (e) {
			this.#fail(e);
			return;
		}
		this.#remapKeys(rel, nextRel);
		await this.#rewriteMentionLinks(this.#folderRemap(rel, nextRel));
		await this.refresh();
	}

	async renameFolder(rel: string, rawName: string) {
		if (!this.folder) return;
		const name = this.#sanitizeName(rawName);
		if (!name) return;
		const parent = rel.includes('/') ? rel.slice(0, rel.lastIndexOf('/')) : null;
		const nextRel = parent ? `${parent}/${name}` : name;
		if (nextRel === rel) return;
		if (this.folders.includes(nextRel)) {
			this.error = 'A folder with this name already exists';
			return;
		}
		const rootAbs = this.folder.replace(/\/+$/, '');
		try {
			await invoke('rename_note', {
				oldPath: `${rootAbs}/${rel}`,
				newPath: `${rootAbs}/${nextRel}`
			});
		} catch (e) {
			this.#fail(e);
			return;
		}
		this.#remapKeys(rel, nextRel);
		await this.#rewriteMentionLinks(this.#folderRemap(rel, nextRel));
		await this.refresh();
	}

	// only empty folders can be deleted; move the notes out first
	async deleteFolder(rel: string) {
		if (!this.folder) return;
		const rootAbs = this.folder.replace(/\/+$/, '');
		try {
			await invoke('delete_folder', { path: `${rootAbs}/${rel}` });
		} catch {
			this.error = 'Folder is not empty — move the notes out first';
			return;
		}
		await this.refresh();
	}

	// expanded/collapsed state and manual orders are keyed by folder path;
	// after a move or rename those keys must follow
	#remapKeys(oldRel: string, newRel: string) {
		const remap = (k: string) =>
			k === oldRel
				? newRel
				: k.startsWith(`${oldRel}/`)
					? `${newRel}/${k.slice(oldRel.length + 1)}`
					: k;
		const expanded: Record<string, boolean> = {};
		for (const [k, v] of Object.entries(this.expandedFolders)) expanded[remap(k)] = v;
		this.expandedFolders = expanded;
		const orders: Record<string, string[]> = {};
		for (const [k, v] of Object.entries(this.#manualOrders)) orders[remap(k)] = v;
		this.#manualOrders = orders;
	}

	async renameNote(path: string, rawTitle: string) {
		const title = rawTitle
			.trim()
			.replace(/[\\/:]/g, '-')
			.replace(/\.md$/i, '');
		if (!title) return;
		const parent = path.split('/').slice(0, -1).join('/');
		const newPath = `${parent}/${title}.md`;
		if (newPath === path) return;
		const taken = [...this.allNotes, ...this.archived].some((n) => n.path === newPath);
		if (taken) {
			this.error = 'A note with this name already exists';
			return;
		}
		try {
			await invoke('rename_note', { oldPath: path, newPath });
		} catch (e) {
			this.#fail(e);
			return;
		}
		if (this.selectedPath === path) this.selectedPath = newPath;
		await this.#rewriteMentionLinks(new Map([[path, newPath]]));
		await this.refresh();
	}

	async archiveNote(path: string) {
		const name = path.split('/').pop();
		if (!name) return;
		try {
			await invoke('rename_note', { oldPath: path, newPath: `${this.archiveDir}/${name}` });
		} catch (e) {
			this.#fail(e);
			return;
		}
		if (this.selectedPath === path) {
			this.selectedPath = null;
			this.content = '';
		}
		await this.#rewriteMentionLinks(new Map([[path, `${this.archiveDir}/${name}`]]));
		await this.refresh();
	}

	async restoreNote(path: string) {
		const name = path.split('/').pop();
		if (!name || !this.folder) return;
		try {
			await invoke('rename_note', {
				oldPath: path,
				newPath: `${this.folder.replace(/\/+$/, '')}/${name}`
			});
		} catch (e) {
			this.#fail(e);
			return;
		}
		if (this.selectedPath === path) {
			this.selectedPath = null;
			this.content = '';
		}
		await this.#rewriteMentionLinks(
			new Map([[path, `${this.folder.replace(/\/+$/, '')}/${name}`]])
		);
		await this.refresh();
	}

	async deleteNote(path: string) {
		// deleting moves the note to the notes trash so it can be restored
		const name = path.split('/').pop();
		if (!name) return;
		try {
			await invoke('rename_note', { oldPath: path, newPath: `${this.trashDir}/${name}` });
		} catch (e) {
			this.#fail(e);
			return;
		}
		if (this.selectedPath === path) {
			this.selectedPath = null;
			this.content = '';
		}
		await this.#rewriteMentionLinks(new Map([[path, `${this.trashDir}/${name}`]]));
		await this.refresh();
	}
}

export const notesState = NotesPageState.get();
