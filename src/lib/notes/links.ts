// link extraction shared by the links index, backlinks and unlinked mentions

export type ExtractedLinks = {
	// absolute note paths referenced through @[label](note:PATH)
	noteTargets: string[];
	// task ids referenced through [label](task:ID)
	taskIds: string[];
	// [[wiki]] / [[Name|alias]] target names (unresolved)
	wikiNames: string[];
	// inline #tags found in the body (frontmatter excluded by the caller)
	inlineTags: string[];
};

const MENTION_RE = /\]\(note:([^)\s]+)\)/g;
const TASK_RE = /\]\(task:([^)\s]+)\)/g;
const WIKI_RE = /\[\[([^\]|]+)(?:\|[^\]]*)?\]\]/g;
const TAG_RE = /(?<![\w#])#([\p{L}\p{N}][\p{L}\p{N}/_-]*)/gu;

export function extractLinks(content: string): ExtractedLinks {
	const noteTargets = new Set<string>();
	const taskIds = new Set<string>();
	const wikiNames = new Set<string>();
	const inlineTags = new Set<string>();
	for (const m of content.matchAll(MENTION_RE)) {
		// block links carry a #L<n> line fragment; the index stores the bare path
		const target = decodeURIComponent(m[1]).replace(/#L\d+$/, '');
		if (target) noteTargets.add(target);
	}
	for (const m of content.matchAll(TASK_RE)) taskIds.add(decodeURIComponent(m[1]));
	for (const m of content.matchAll(WIKI_RE)) {
		const name = m[1].trim();
		if (name) wikiNames.add(name);
	}
	// headings and frontmatter delimiters must not become tags
	const noFrontmatter = content.replace(/^---\n[\s\S]*?\n---\n/, '');
	for (const line of noFrontmatter.split('\n')) {
		if (/^#{1,6}\s/.test(line)) continue;
		for (const m of line.matchAll(TAG_RE)) inlineTags.add(m[1]);
	}
	return {
		noteTargets: [...noteTargets],
		taskIds: [...taskIds],
		wikiNames: [...wikiNames],
		inlineTags: [...inlineTags]
	};
}

// decode a wiki name into a filesystem-safe lookup key (matches file names)
export function wikiToFileName(name: string): string {
	const clean = name.replace(/[/\\:]/g, '-').trim();
	return clean.toLowerCase().endsWith('.md') ? clean : `${clean}.md`;
}
