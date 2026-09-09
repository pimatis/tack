// minimal markdown to html renderer - handles common syntax

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#39;');
}

function renderInline(text: string): string {
	let result = escapeHtml(text);

	// inline code
	result = result.replace(/`([^`]+)`/g, '<code>$1</code>');

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

	// links
	result = result.replace(
		/\[([^\]]+)\]\(([^)\s]+)\)/g,
		'<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>'
	);

	return result;
}

export function renderMarkdown(md: string): string {
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
			html.push(
				`<pre class="group relative"><button type="button" class="copy-code-btn absolute right-2 flex h-7 w-7 items-center justify-center rounded-md border border-border bg-popover/90 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:text-foreground" style="top: min(calc(50% - 14px), 2rem)" aria-label="Copy code"><svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" d="M9 2a2 2 0 0 0-2 2v2h2V4h11v11h-2v2h2a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM4 7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2z"/></svg></button><code${lang ? ` class="language-${escapeHtml(lang)}"` : ''}>${escapeHtml(code.join('\n'))}</code></pre>`
			);
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
				`<h${level} id="heading-${headingCount++}">${renderInline(headingMatch[2])}</h${level}>`
			);
			i++;
			continue;
		}

		// blockquote
		if (line.trim().startsWith('>')) {
			const quote: string[] = [];
			while (i < lines.length && lines[i].trim().startsWith('>')) {
				quote.push(lines[i].trim().replace(/^>\s?/, ''));
				i++;
			}
			html.push(`<blockquote>${renderInline(quote.join(' '))}</blockquote>`);
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
						`<li class="flex list-none! items-start gap-2"><span data-todo="${i}" data-checked="${checked}" class="mt-0.5 inline-block size-4 shrink-0"></span><span class="${checked ? 'text-muted-foreground line-through' : ''}">${renderInline(task[2])}</span></li>`
					);
				} else {
					items.push(`<li>${renderInline(raw.replace(/^\s*[-*+]\s+/, ''))}</li>`);
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
				items.push(`<li>${renderInline(lines[i].replace(/^\s*\d+\.\s+/, ''))}</li>`);
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
			!/^\s*\d+\.\s+/.test(lines[i]) &&
			!/^---+\s*$/.test(lines[i])
		) {
			para.push(lines[i]);
			i++;
		}
		html.push(`<p>${renderInline(para.join('\n')).replaceAll('\n', '<br />')}</p>`);
	}

	return html.join('\n');
}
