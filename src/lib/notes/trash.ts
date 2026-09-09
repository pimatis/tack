import { invoke } from '@tauri-apps/api/core';
import type { NoteInfo } from './notesState.svelte';

// helpers for the trash page; work on <folder>/trash where deleted notes go

function trashDir(folder: string): string {
	return `${folder.replace(/\/+$/, '')}/trash`;
}

export async function listTrashedNotes(folder: string): Promise<NoteInfo[]> {
	// missing trash folder just means no notes were deleted
	return invoke<NoteInfo[]>('list_notes', { dir: trashDir(folder) }).catch(() => []);
}

async function noteExists(dir: string, name: string): Promise<boolean> {
	const notes = await invoke<NoteInfo[]>('list_notes', { dir }).catch(() => []);
	return notes.some((n) => n.name === name);
}

export async function restoreTrashedNote(folder: string, name: string): Promise<void> {
	const base = folder.replace(/\/+$/, '');
	const stem = name.replace(/\.md$/, '');
	// never overwrite an existing note: pick the first free name
	let target = `${base}/${name}`;
	for (let i = 2; await noteExists(base, target.split('/').pop()!); i++) {
		target = `${base}/${stem} ${i}.md`;
	}
	await invoke('rename_note', { oldPath: `${trashDir(folder)}/${name}`, newPath: target });
}

export async function purgeNote(folder: string, name: string): Promise<void> {
	await invoke('delete_note', { path: `${trashDir(folder)}/${name}` });
}

export async function purgeAllNotes(folder: string): Promise<void> {
	const trashed = await listTrashedNotes(folder);
	for (const note of trashed) {
		await invoke('delete_note', { path: note.path }).catch(() => {});
	}
}
