/**
 * Notes data bridge: tauri invoke on desktop, live-server http endpoints in
 * the browser. Same command names, so notesState stays transport-agnostic.
 */
import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '$lib/db/client';
import { requireLiveAuth } from '$lib/live/auth.service';

type Args = Record<string, unknown>;

async function http<T>(url: string, init?: RequestInit): Promise<T> {
	const res = await fetch(url, init);
	if (res.status === 401) {
		await requireLiveAuth();
		return http(url, init);
	}
	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(body.error ?? 'Request failed');
	}
	return res.json() as Promise<T>;
}

async function post<T>(url: string, body: Args): Promise<T> {
	return http<T>(url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});
}

export async function notesInvoke<T>(cmd: string, args: Args = {}): Promise<T> {
	if (isTauri()) return invoke<T>(cmd, args);

	switch (cmd) {
		case 'read_notes_deep':
		case 'list_notes_deep':
			return http<T>('/api/notes/deep');
		case 'list_note_folders':
			return http<T>(`/api/notes/folders?dir=${encodeURIComponent(args.dir as string)}`);
		case 'list_notes':
			return http<T>(`/api/notes/list?dir=${encodeURIComponent(args.dir as string)}`);
		case 'read_file':
			return http<T>(`/api/notes/file?path=${encodeURIComponent(args.path as string)}`);
		case 'note_info':
			return http<T>(`/api/notes/info?path=${encodeURIComponent(args.path as string)}`);
		case 'write_file':
			return post<T>('/api/notes/write', { path: args.path, content: args.content });
		case 'save_note_with_history':
			return post<T>('/api/notes/save', {
				path: args.path,
				content: args.content,
				key: args.key,
				keep: args.keep
			});
		case 'rename_note':
			return post<T>('/api/notes/rename', { from: args.oldPath, to: args.newPath });
		case 'delete_note':
			return post<T>('/api/notes/delete', { path: args.path });
		case 'create_folder':
			return post<T>('/api/notes/folder', { dir: args.dir, name: args.name });
		case 'delete_folder':
			return post<T>('/api/notes/folder-delete', { path: args.path });
		case 'write_binary_file':
			return post<T>('/api/notes/binary', { path: args.path, bytes: args.bytes });
		default:
			throw new Error(`Command ${cmd} is not available in live mode`);
	}
}

// browser clients load attachments through the live server
export function noteAssetUrl(path: string): string {
	return `/api/notes/asset?path=${encodeURIComponent(path)}`;
}

// the desktop app's configured notes folder (browser/live mode only)
export async function notesRoot(): Promise<string> {
	const { root } = await http<{ root: string }>('/api/notes/root');
	return root;
}
