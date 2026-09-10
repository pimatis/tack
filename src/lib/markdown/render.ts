// markdown to html renderer - common syntax plus tables, callouts, wiki
// links, tags, images and lightweight code highlighting (no dependencies)

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#39;');
}

export type RenderOptions = {
	// resolves a relative image path to something the webview can load
	resolveAsset?: (rel: string) => string;
};

const CALLOUT_TYPES = new Set(['note', 'info', 'tip', 'success', 'warning', 'danger', 'error']);

const KEYWORDS: Record<string, string[]> = {
	js: [
		'const',
		'let',
		'var',
		'function',
		'return',
		'if',
		'else',
		'for',
		'while',
		'import',
		'from',
		'export',
		'default',
		'class',
		'extends',
		'new',
		'await',
		'async',
		'try',
		'catch',
		'finally',
		'throw',
		'typeof',
		'instanceof',
		'in',
		'of',
		'switch',
		'case',
		'break',
		'continue',
		'this',
		'super',
		'yield',
		'static',
		'get',
		'set',
		'delete',
		'void'
	],
	ts: [
		'const',
		'let',
		'var',
		'function',
		'return',
		'if',
		'else',
		'for',
		'while',
		'import',
		'from',
		'export',
		'default',
		'class',
		'extends',
		'implements',
		'interface',
		'type',
		'new',
		'await',
		'async',
		'try',
		'catch',
		'finally',
		'throw',
		'typeof',
		'instanceof',
		'in',
		'of',
		'switch',
		'case',
		'break',
		'continue',
		'this',
		'super',
		'yield',
		'static',
		'public',
		'private',
		'protected',
		'readonly',
		'enum',
		'declare',
		'namespace',
		'as',
		'satisfies'
	],
	rust: [
		'fn',
		'let',
		'mut',
		'const',
		'static',
		'struct',
		'enum',
		'trait',
		'impl',
		'pub',
		'use',
		'mod',
		'match',
		'if',
		'else',
		'loop',
		'while',
		'for',
		'in',
		'return',
		'break',
		'continue',
		'where',
		'async',
		'await',
		'move',
		'ref',
		'dyn',
		'crate',
		'self',
		'super',
		'type',
		'unsafe',
		'as'
	],
	python: [
		'def',
		'class',
		'return',
		'if',
		'elif',
		'else',
		'for',
		'while',
		'import',
		'from',
		'as',
		'with',
		'try',
		'except',
		'finally',
		'raise',
		'lambda',
		'yield',
		'global',
		'nonlocal',
		'pass',
		'break',
		'continue',
		'and',
		'or',
		'not',
		'in',
		'is',
		'assert',
		'async',
		'await',
		'del'
	],
	json: ['true', 'false', 'null'],
	bash: [
		'if',
		'then',
		'else',
		'elif',
		'fi',
		'for',
		'while',
		'do',
		'done',
		'case',
		'esac',
		'function',
		'return',
		'export',
		'local',
		'source',
		'echo',
		'exit'
	],
	css: ['important', 'media', 'keyframes', 'supports', 'import', 'from'],
	sql: [
		'select',
		'from',
		'where',
		'insert',
		'into',
		'values',
		'update',
		'set',
		'delete',
		'create',
		'table',
		'drop',
		'alter',
		'join',
		'left',
		'right',
		'inner',
		'outer',
		'on',
		'group',
		'by',
		'order',
		'having',
		'limit',
		'offset',
		'as',
		'and',
		'or',
		'not',
		'null',
		'primary',
		'key',
		'foreign',
		'references'
	]
};

function keywordSet(lang: string): Set<string> | null {
	const l = lang.toLowerCase();
	const map: Record<string, string> = {
		javascript: 'js',
		js: 'js',
		jsx: 'js',
		mjs: 'js',
		typescript: 'ts',
		ts: 'ts',
		tsx: 'ts',
		rs: 'rust',
		rust: 'rust',
		py: 'python',
		python: 'python',
		json: 'json',
		jsonc: 'json',
		sh: 'bash',
		bash: 'bash',
		shell: 'bash',
		zsh: 'bash',
		css: 'css',
		scss: 'css',
		sql: 'sql',
		sqlite: 'sql'
	};
	const family = map[l];
	if (!family) return null;
	return new Set(KEYWORDS[family]);
}

// hash comments for script/config languages, // and /* */ for the rest
function hashComments(lang: string): boolean {
	return /^(py|python|sh|bash|shell|zsh|yaml|yml|toml|ruby|rb|conf|ini)$/.test(lang.toLowerCase());
}

function highlightCode(code: string, lang: string): string {
	const keywords = keywordSet(lang);
	if (!keywords) return escapeHtml(code);
	const hash = hashComments(lang);
	const re = new RegExp(
		`${hash ? '(#[^\\n]*)' : '(\\/\\/[^\\n]*|\\/\\*[\\s\\S]*?\\*\\/)'}|("(?:[^"\\\\\\n]|\\\\.)*"|'(?:[^'\\\\\\n]|\\\\.)*'|\`(?:[^\`\\\\]|\\\\.)*\`)|(\\b\\d+(?:\\.\\d+)?\\b)|([A-Za-z_$][\\w$]*)`,
		'g'
	);
	let out = '';
	let last = 0;
	for (const m of code.matchAll(re)) {
		out += escapeHtml(code.slice(last, m.index));
		const [text, comment, str, num, ident] = m;
		if (comment) out += `<span class="tok-c">${escapeHtml(text)}</span>`;
		else if (str) out += `<span class="tok-s">${escapeHtml(text)}</span>`;
		else if (num) out += `<span class="tok-n">${escapeHtml(text)}</span>`;
		else if (ident && keywords.has(ident)) out += `<span class="tok-k">${escapeHtml(text)}</span>`;
		else if (ident) {
			// identifier followed by an opening paren reads as a call
			const after = code.slice((m.index ?? 0) + text.length).match(/^\s*\(/);
			out += after ? `<span class="tok-f">${escapeHtml(text)}</span>` : escapeHtml(text);
		} else out += escapeHtml(text);
		last = (m.index ?? 0) + text.length;
	}
	out += escapeHtml(code.slice(last));
	return out;
}

function renderInline(text: string, opts?: RenderOptions): string {
	let result = escapeHtml(text);

	// inline code is masked so later rules cannot format its content
	const codeSpans: string[] = [];
	result = result.replace(/`([^`]+)`/g, (_, code) => {
		codeSpans.push(`<code>${code}</code>`);
		return `\u0000${codeSpans.length - 1}\u0000`;
	});

	// images: ![alt](src) - http(s) stays, relative paths go through the resolver
	result = result.replace(/!\[([^\]]*)\]\(([^)\s]+)\)/g, (_, alt, src) => {
		const resolved = opts?.resolveAsset && !/^https?:\/\//.test(src) ? opts.resolveAsset(src) : src;
		return `<img src="${resolved}" alt="${alt}" loading="lazy" class="max-w-full rounded-md border border-border" />`;
	});

	// wiki links: [[Name]] or [[Name|alias]]; resolved by the preview host
	result = result.replace(
		/\[\[([^\]|]+)(?:\|([^\]]*))?\]\]/g,
		(_, name, alias) =>
			`<a href="#" data-wiki="${name.trim()}" class="wiki-link underline decoration-border underline-offset-2 hover:decoration-foreground">${alias?.trim() || name}</a>`
	);

	// links
	result = result.replace(
		/\[([^\]]+)\]\(([^)\s]+)\)/g,
		'<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>'
	);

	// inline tags; a letter somewhere in the name keeps &#39; entities and
	// heading markers out
	result = result.replace(
		/(?<![\w#&])#(?=[\p{L}\p{N}/_-]*\p{L})([\p{L}\p{N}/_-]+)/gu,
		'<span data-tag="$1" class="note-tag rounded-sm bg-primary/10 px-1 text-primary">#$1</span>'
	);

	// bold
	result = result.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
	result = result.replace(/__([^_]+)__/g, '<strong>$1</strong>');

	// italic
	result = result.replace(/\*([^*]+)\*/g, '<em>$1</em>');
	result = result.replace(/_([^_]+)_/g, '<em>$1</em>');

	// strikethrough
	result = result.replace(/~~([^~]+)~~/g, '<del>$1</del>');

	// highlight
	result = result.replace(
		/==([^=]+)==/g,
		'<mark class="rounded-sm bg-primary/20 px-0.5 text-inherit">$1</mark>'
	);

	// underline (++text++ convention used by several markdown editors)
	result = result.replace(/\+\+([^+]+)\+\+/g, '<u>$1</u>');

	// restore masked code spans
	// eslint-disable-next-line no-control-regex
	result = result.replace(/\u0000(\d+)\u0000/g, (_, i) => codeSpans[Number(i)] ?? '');

	return result;
}

// consecutive | rows; the second row must be a --- separator
function isTableStart(lines: string[], i: number): boolean {
	if (!/^\s*\|.+\|\s*$/.test(lines[i])) return false;
	return /^\s*\|[\s:-]+\|\s*$/.test(lines[i + 1] ?? '');
}

function renderTable(
	lines: string[],
	start: number,
	opts?: RenderOptions
): { html: string; next: number } {
	const splitRow = (row: string) =>
		row
			.trim()
			.replace(/^\|/, '')
			.replace(/\|$/, '')
			.split('|')
			.map((c) => c.trim());
	const headers = splitRow(lines[start]);
	let i = start + 2; // skip the separator row
	const rows: string[][] = [];
	while (i < lines.length && /^\s*\|.*\|\s*$/.test(lines[i])) {
		rows.push(splitRow(lines[i]));
		i++;
	}
	const head = headers.map((h) => `<th>${renderInline(h, opts)}</th>`).join('');
	const body = rows
		.map((row) => `<tr>${row.map((c) => `<td>${renderInline(c, opts)}</td>`).join('')}</tr>`)
		.join('');
	return {
		html: `<div class="overflow-x-auto"><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`,
		next: i
	};
}

// blockquote starting with > [!type] becomes a callout box
function calloutOf(line: string): { type: string; title: string } | null {
	const m = line.match(/^>\s*\[!([\w-]+)\]\s*(.*)$/);
	if (!m) return null;
	const type = m[1].toLowerCase();
	if (!CALLOUT_TYPES.has(type)) return null;
	return { type, title: m[2].trim() };
}

export function renderMarkdown(md: string, opts?: RenderOptions): string {
	if (!md?.trim()) return '';

	const lines = md.split('\n');
	const html: string[] = [];
	let i = 0;
	// heading ids are used by the outline to scroll to a heading
	let headingCount = 0;

	while (i < lines.length) {
		const line = lines[i];

		// code block
		if (line.trim().startsWith('```')) {
			const lang = line.trim().slice(3).trim();
			const code: string[] = [];
			i++;
			while (i < lines.length && !lines[i].trim().startsWith('```')) {
				code.push(lines[i]);
				i++;
			}
			i++;
			const inner = highlightCode(code.join('\n'), lang);
			html.push(
				`<pre class="group relative"><button type="button" class="copy-code-btn absolute right-2 flex h-7 w-7 items-center justify-center rounded-md border border-border bg-popover/90 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground" style="top: min(calc(50% - 14px), 2rem)" aria-label="Copy code"><svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" d="M9 2a2 2 0 0 0-2 2v2h2V4h11v11h-2v2h2a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM4 7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2z"/></svg></button><code${lang ? ` class="language-${escapeHtml(lang)}"` : ''}>${inner}</code></pre>`
			);
			continue;
		}

		// tables
		if (isTableStart(lines, i)) {
			const table = renderTable(lines, i, opts);
			html.push(table.html);
			i = table.next;
			continue;
		}

		// horizontal rule
		if (/^---+\s*$/.test(line)) {
			html.push('<hr />');
			i++;
			continue;
		}

		// headings
		const headingMatch = line.match(/^(#{1,6})\s+(.*)$/);
		if (headingMatch) {
			const level = headingMatch[1].length;
			html.push(
				`<h${level} id="heading-${headingCount++}">${renderInline(headingMatch[2], opts)}</h${level}>`
			);
			i++;
			continue;
		}

		// blockquote or callout
		if (line.trim().startsWith('>')) {
			const callout = calloutOf(line.trim());
			const quote: string[] = [];
			while (i < lines.length && lines[i].trim().startsWith('>')) {
				quote.push(lines[i].trim().replace(/^>\s?/, ''));
				i++;
			}
			if (callout) {
				quote.shift(); // drop the [!type] marker line
				const body = quote
					.filter((l) => l.trim() !== '')
					.map((l) => `<p>${renderInline(l, opts)}</p>`)
					.join('');
				const title = callout.title || callout.type.charAt(0).toUpperCase() + callout.type.slice(1);
				html.push(
					`<div class="callout callout-${callout.type}"><div class="callout-title">${renderInline(title, opts)}</div>${body}</div>`
				);
			} else {
				html.push(`<blockquote>${renderInline(quote.join(' '), opts)}</blockquote>`);
			}
			continue;
		}

		// unordered list, including github-style task items
		if (/^\s*[-*+]\s+/.test(line)) {
			const items: string[] = [];
			while (i < lines.length && /^\s*[-*+]\s+/.test(lines[i])) {
				const raw = lines[i];
				const task = raw.match(/^\s*[-*+]\s+\[( |x|X)\]\s+(.*)$/);
				if (task) {
					const checked = task[1].toLowerCase() === 'x';
					// placeholder span; MarkdownRenderer mounts the ui Checkbox here,
					// data-line points at the raw markdown line so toggling can rewrite it
					items.push(
						`<li class="flex list-none! items-start gap-2"><span data-todo="${i}" data-checked="${checked}" class="mt-0.5 inline-block size-4 shrink-0"></span><span class="${checked ? 'text-muted-foreground line-through' : ''}">${renderInline(task[2], opts)}</span></li>`
					);
				} else {
					items.push(`<li>${renderInline(raw.replace(/^\s*[-*+]\s+/, ''), opts)}</li>`);
				}
				i++;
			}
			html.push(`<ul>${items.join('')}</ul>`);
			continue;
		}

		// ordered list
		if (/^\s*\d+\.\s+/.test(line)) {
			const items: string[] = [];
			while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
				items.push(`<li>${renderInline(lines[i].replace(/^\s*\d+\.\s+/, ''), opts)}</li>`);
				i++;
			}
			html.push(`<ol>${items.join('')}</ol>`);
			continue;
		}

		// empty line
		if (line.trim() === '') {
			i++;
			continue;
		}

		// paragraph; single newlines inside are kept as visible line breaks
		const para: string[] = [];
		while (
			i < lines.length &&
			lines[i].trim() !== '' &&
			!lines[i].trim().startsWith('```') &&
			!lines[i].trim().startsWith('>') &&
			!/^#{1,6}\s+/.test(lines[i]) &&
			!/^\s*[-*+]\s+/.test(lines[i]) &&
			!isTableStart(lines, i) &&
			!/^\s*\d+\.\s+/.test(lines[i]) &&
			!/^---+\s*$/.test(lines[i])
		) {
			para.push(lines[i]);
			i++;
		}
		html.push(`<p>${renderInline(para.join('\n'), opts).replaceAll('\n', '<br />')}</p>`);
	}

	return html.join('\n');
}
