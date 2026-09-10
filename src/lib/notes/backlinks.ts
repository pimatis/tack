import { invoke } from '@tauri-apps/api/core';

export type Backlink = { path: string; name: string };

type NoteFullInfo = { path: string; name: string; content: string };

// notes whose content links to the target note through a @[label](note:PATH)
// mention; a plain content scan is enough since every save reindexes anyway
export async function getBacklinks(folder: string, targetPath: string): Promise<Backlink[]> {
	const all = await invoke<NoteFullInfo[]>('read_notes_deep', { dir: folder }).catch(() => []);
	const needle = `](note:${encodeURIComponent(targetPath)})`;
	return all
		.filter((n) => n.path !== targetPath && n.content.includes(needle))
		.map(({ path, name }) => ({ path, name }));
}
