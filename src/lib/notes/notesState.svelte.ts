import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { isTauri } from '$lib/db/client';
import { reindexNotes, indexNote } from './search';

export type NoteInfo = { name: string; path: string; modified: number };

const FOLDER_KEY = 'tack-notes-folder';
const TAB_KEY = 'tack-notes-tab';
const SORT_KEY = 'tack-notes-sort';

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
	archived = $state<NoteInfo[]>([]);
	showArchived = $state(false);
	pinned = $state<string[]>([]);
	sortBy = $state<'modified' | 'name'>('modified');
	visitCounts = $state<Record<string, number>>({});
	selectedPath = $state<string | null>(null);
	content = $state('');
	preview = $state(false);
	loading = $state(false);
	saving = $state(false);
	error = $state<string | null>(null);

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
		this.sortBy = localStorage.getItem(SORT_KEY) === 'name' ? 'name' : 'modified';
		if (this.folder) await this.refresh();
	}

	async refresh() {
		if (!this.folder) return;
		this.loading = true;
		this.error = null;
		try {
			this.notes = await invoke<NoteInfo[]>('list_notes', { dir: this.folder });
			// missing archive folder just means nothing is archived yet
			this.archived = await invoke<NoteInfo[]>('list_notes', { dir: this.archiveDir }).catch(
				() => []
			);
			this.#loadPins();
			this.#loadVisits();
			// rebuild the search index so external edits stay searchable; best effort
			try {
				const full = await invoke<{ path: string; name: string; content: string }[]>('read_notes', {
					dir: this.folder
				});
				await reindexNotes(full);
			} catch {
				// search stays stale rather than breaking the notes list
			}
		} catch (e) {
			this.error = String(e);
			this.notes = [];
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
				? stored.filter((n: string) => this.notes.some((note) => note.name === n))
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
		return [...this.notes]
			.filter((n) => (this.visitCounts[n.name] ?? 0) > 0)
			.sort((a, b) => (this.visitCounts[b.name] ?? 0) - (this.visitCounts[a.name] ?? 0))
			.slice(0, 4);
	}

	setSort(sort: 'modified' | 'name') {
		this.sortBy = sort;
		localStorage.setItem(SORT_KEY, sort);
	}

	// pinned notes first, then by the chosen sort order
	get sortedNotes(): NoteInfo[] {
		const list = [...this.notes];
		list.sort((a, b) =>
			this.sortBy === 'name' ? a.name.localeCompare(b.name) : b.modified - a.modified
		);
		const isPinned = (n: NoteInfo) => this.pinned.includes(n.name);
		return [...list.filter(isPinned), ...list.filter((n) => !isPinned(n))];
	}

	get archiveDir(): string {
		return this.folder ? `${this.folder.replace(/\/+$/, '')}/archive` : '';
	}

	get trashDir(): string {
		return this.folder ? `${this.folder.replace(/\/+$/, '')}/trash` : '';
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
		this.showArchived = false;
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

	async createNote() {
		if (!this.folder) return;
		this.flushPendingSave();
		this.showArchived = false;
		// first name that does not exist yet: Untitled.md, Untitled 2.md, ...
		let name = 'Untitled.md';
		for (let i = 2; this.notes.some((n) => n.name === name); i++) name = `Untitled ${i}.md`;
		const path = `${this.folder.replace(/\/+$/, '')}/${name}`;
		try {
			await invoke('write_file', { path, content: `# ${name.replace(/\.md$/, '')}\n\n` });
		} catch (e) {
			this.error = String(e);
			return;
		}
		await this.refresh();
		await this.openNote(path);
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
		const taken = [...this.notes, ...this.archived].some((n) => n.path === newPath);
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
