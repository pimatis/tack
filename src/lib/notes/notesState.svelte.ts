import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { isTauri } from '$lib/db/client';
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
	content = $state('');
	preview = $state(false);
	loading = $state(false);
	saving = $state(false);
	error = $state<string | null>(null);

	// manual drag order per container ('root' or a relative folder path)
	#manualOrders: Record<string, string[]> = {};

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
	}

	async refresh() {
		if (!this.folder) return;
		this.loading = true;
		this.error = null;
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
			this.visitCounts = stored && typeof stored === 'object' ? stored : {};
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
		await this.refresh();
	}

	async openNote(path: string) {
		if (path === this.selectedPath) return;
		this.flushPendingSave();
		this.selectedPath = path;
		this.preview = false;
		this.content = '';
		const name = path.split('/').pop();
		if (name) this.noteOpened(name);
		try {
			this.content = await invoke<string>('read_file', { path });
		} catch (e) {
			this.error = String(e);
		}
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
			const name = this.selectedPath.split('/').pop() ?? '';
			await indexNote(this.selectedPath, name, this.content).catch(() => {});
		} catch (e) {
			this.error = String(e);
		} finally {
			this.saving = false;
		}
	}

	// sanitize a user-typed name into a safe file/folder name
	#sanitizeName(raw: string): string | null {
		const name = raw
			.trim()
			.replace(/[/\\:]/g, '-')
			.replace(/\.md$/i, '');
		return name && name !== '.' && name !== '..' ? name : null;
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
			this.error = String(e);
			return;
		}
		await this.refresh();
		await this.openNote(path);
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
			this.error = String(e);
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
			this.error = String(e);
			return;
		}
		// the note left its container; drop it from every manual order list
		for (const key of Object.keys(this.#manualOrders)) {
			this.#manualOrders[key] = this.#manualOrders[key].filter((n) => n !== name);
		}
		if (this.selectedPath === path) this.selectedPath = newPath;
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
			this.error = String(e);
			return;
		}
		this.#remapKeys(rel, nextRel);
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
			this.error = String(e);
			return;
		}
		this.#remapKeys(rel, nextRel);
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
			this.error = String(e);
			return;
		}
		if (this.selectedPath === path) this.selectedPath = newPath;
		await this.refresh();
	}

	async archiveNote(path: string) {
		const name = path.split('/').pop();
		if (!name) return;
		try {
			await invoke('rename_note', { oldPath: path, newPath: `${this.archiveDir}/${name}` });
		} catch (e) {
			this.error = String(e);
			return;
		}
		if (this.selectedPath === path) {
			this.selectedPath = null;
			this.content = '';
		}
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
			this.error = String(e);
			return;
		}
		if (this.selectedPath === path) {
			this.selectedPath = null;
			this.content = '';
		}
		await this.refresh();
	}

	async deleteNote(path: string) {
		// deleting moves the note to the notes trash so it can be restored
		const name = path.split('/').pop();
		if (!name) return;
		try {
			await invoke('rename_note', { oldPath: path, newPath: `${this.trashDir}/${name}` });
		} catch (e) {
			this.error = String(e);
			return;
		}
		if (this.selectedPath === path) {
			this.selectedPath = null;
			this.content = '';
		}
		await this.refresh();
	}
}

export const notesState = NotesPageState.get();
