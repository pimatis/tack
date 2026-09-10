// minimal yaml frontmatter support: enough for tags and simple metadata
// without pulling a full yaml parser

export type NoteFrontmatter = Record<string, string | string[]>;

// split `---` frontmatter from the markdown body
export function splitFrontmatter(text: string): { data: NoteFrontmatter; body: string } {
	const match = text.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
	if (!match) return { data: {}, body: text };
	return { data: parseYaml(match[1]), body: text.slice(match[0].length) };
}

// serialize frontmatter back to text; body keeps its original content
export function buildFrontmatter(data: NoteFrontmatter, body: string): string {
	const keys = Object.keys(data).filter((k) => data[k] !== undefined && data[k] !== '');
	if (keys.length === 0) return body;
	const lines: string[] = ['---'];
	for (const key of keys) {
		const value = data[key];
		if (Array.isArray(value)) {
			if (value.length === 0) continue;
			lines.push(`${key}:`);
			for (const item of value) lines.push(`  - ${item}`);
		} else {
			lines.push(`${key}: ${value}`);
		}
	}
	lines.push('---', '');
	return `${lines.join('\n')}\n${body}`;
}

function parseYaml(raw: string): NoteFrontmatter {
	const data: NoteFrontmatter = {};
	let currentKey: string | null = null;
	for (const line of raw.split(/\r?\n/)) {
		if (!line.trim()) continue;
		const listItem = line.match(/^\s+-\s+(.*)$/);
		if (listItem && currentKey) {
			const existing = data[currentKey];
			const list = Array.isArray(existing) ? existing : [];
			list.push(cleanValue(listItem[1]));
			data[currentKey] = list;
			continue;
		}
		const pair = line.match(/^([\w-]+):\s*(.*)$/);
		if (!pair) continue;
		currentKey = pair[1];
		if (pair[2].trim() === '') {
			data[currentKey] = [];
		} else if (pair[2].startsWith('[') && pair[2].endsWith(']')) {
			data[currentKey] = pair[2]
				.slice(1, -1)
				.split(',')
				.map((item) => cleanValue(item))
				.filter((item) => item !== '');
		} else {
			data[currentKey] = cleanValue(pair[2]);
		}
	}
	return data;
}

function cleanValue(raw: string): string {
	return raw.trim().replace(/^["']|["']$/g, '');
}

// tags from the frontmatter only; inline #tags are handled separately
export function tagsOf(data: NoteFrontmatter): string[] {
	const tags = data.tags;
	if (Array.isArray(tags)) return tags.filter((t): t is string => typeof t === 'string');
	if (typeof tags === 'string') {
		return tags
			.split(',')
			.map((t) => t.trim())
			.filter(Boolean);
	}
	return [];
}

export function withTags(data: NoteFrontmatter, tags: string[]): NoteFrontmatter {
	const next: NoteFrontmatter = { ...data, tags };
	return next;
}

// add or remove one tag in the note text, rewriting frontmatter around the body
export function addTag(text: string, tag: string): string {
	const { data, body } = splitFrontmatter(text);
	const tags = tagsOf(data);
	if (!tags.includes(tag)) tags.push(tag);
	return buildFrontmatter(withTags(data, tags), body);
}

export function removeTag(text: string, tag: string): string {
	const { data, body } = splitFrontmatter(text);
	const tags = tagsOf(data).filter((t) => t !== tag);
	return buildFrontmatter(withTags(data, tags), body);
}
