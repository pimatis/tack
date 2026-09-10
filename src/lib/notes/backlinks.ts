import { invoke } from '@tauri-apps/api/core';
import { getLinkingNotes } from './search';
import { wikiToFileName } from './links';

export type Backlink = { path: string; name: string };

// notes linking to the target note, resolved from the links index:
// mention links match on the exact path, wiki links on the file name
export async function getBacklinks(folder: string, targetPath: string): Promise<Backlink[]> {
	const results = new Map<string, Backlink>();
	for (const row of await getLinkingNotes(targetPath).catch(() => [])) {
		if (row.path !== targetPath) results.set(row.path, { path: row.path, name: row.name });
	}
	const fileName = targetPath.split('/').pop() ?? '';
	for (const row of await getLinkingNotes(`wiki:${stripMd(fileName)}`).catch(() => [])) {
		if (row.path !== targetPath) results.set(row.path, { path: row.path, name: row.name });
	}
	return [...results.values()];
}

// notes that mention the target's name in plain text without linking it;
// the content scan is only needed for the open note, not on every refresh
export async function getUnlinkedMentions(folder: string, targetPath: string): Promise<Backlink[]> {
	type NoteFull = { path: string; name: string; content: string };
	const all = await invoke<NoteFull[]>('read_notes_deep', { dir: folder }).catch(() => []);
	const stem = stripMd(targetPath.split('/').pop() ?? '');
	if (!stem) return [];
	const linked = new Set(
		all
			.filter((n) => n.content.includes(`](note:${encodeURIComponent(targetPath)})`))
			.map((n) => n.path)
	);
	return all
		.filter(
			(n) =>
				n.path !== targetPath &&
				!linked.has(n.path) &&
				n.content.toLowerCase().includes(stem.toLowerCase())
		)
		.map(({ path, name }) => ({ path, name }));
}

function stripMd(name: string): string {
	return wikiToFileName(name).replace(/\.md$/i, '');
}
