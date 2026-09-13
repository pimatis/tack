// linked notes and unlinked mentions for one target note, computed from note
// contents in a single scan; one source of truth keeps the two lists disjoint
import { notesInvoke } from './liveNotes';
import { extractLinks, wikiToFileName } from './links';

export type Backlink = { path: string; name: string };

export type NoteLinkPanels = { backlinks: Backlink[]; unlinked: Backlink[] };

type NoteFull = { path: string; name: string; content: string };

function escapeRegExp(text: string): string {
	return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function fileNameOf(path: string): string {
	return (path.split('/').pop() ?? '').toLowerCase();
}

// does the note reference the target through any link syntax? mention links
// match on the exact path (block links' #L fragments are stripped by the
// extractor), wiki links resolve by file name like the preview does
function linksToTarget(content: string, targetPath: string): boolean {
	const links = extractLinks(content);
	if (links.noteTargets.includes(targetPath)) return true;
	const fileName = fileNameOf(targetPath);
	return links.wikiNames.some((n) => wikiToFileName(n).toLowerCase() === fileName);
}

// plain-text mention of the target's name, not glued to other words. when the
// matched text extends into another existing note's full name ("Untitled 3"
// while looking for "Untitled"), it refers to that note and doesn't count here
function mentionsTarget(content: string, targetPath: string, otherStems: string[]): boolean {
	const stem = fileNameOf(targetPath).replace(/\.md$/i, '');
	if (!stem) return false;
	const re = new RegExp(`(?<![\\p{L}\\p{N}_])${escapeRegExp(stem)}`, 'giu');
	for (const m of content.matchAll(re)) {
		const at = m.index + m[0].length;
		const rest = content.slice(at).toLowerCase();
		const refersToOther = otherStems.some((s) => {
			const remainder = s.toLowerCase().slice(stem.length);
			if (!remainder || !rest.startsWith(remainder)) return false;
			// the longer name must also end on a word boundary
			const after = rest[remainder.length];
			return !after || !/[\p{L}\p{N}_]/u.test(after);
		});
		if (!refersToOther) return true;
	}
	return false;
}

// every note that links to the target, plus notes that only mention its name
// in plain text; both lists are disjoint by construction
export async function getLinkPanels(folder: string, targetPath: string): Promise<NoteLinkPanels> {
	const all = await notesInvoke<NoteFull[]>('read_notes_deep', { dir: folder }).catch(() => []);
	// name stems of every other note, used to tell "mentions Untitled 3" apart
	// from "mentions Untitled"
	const otherStems = [
		...new Set(
			all.filter((n) => n.path !== targetPath).map((n) => fileNameOf(n.name).replace(/\.md$/i, ''))
		)
	];
	const backlinks: Backlink[] = [];
	const unlinked: Backlink[] = [];
	for (const note of all) {
		if (note.path === targetPath) continue;
		if (linksToTarget(note.content, targetPath)) {
			backlinks.push({ path: note.path, name: note.name });
		} else if (mentionsTarget(note.content, targetPath, otherStems)) {
			unlinked.push({ path: note.path, name: note.name });
		}
	}
	const byName = (a: Backlink, b: Backlink) => a.name.localeCompare(b.name);
	return { backlinks: backlinks.sort(byName), unlinked: unlinked.sort(byName) };
}
