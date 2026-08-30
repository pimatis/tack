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
				`<pre><code${lang ? ` class="language-${escapeHtml(lang)}"` : ''}>${escapeHtml(code.join('\n'))}</code></pre>`
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
			html.push(`<h${level}>${renderInline(headingMatch[2])}</h${level}>`);
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

		// unordered list
		if (/^\s*[-*+]\s+/.test(line)) {
			const items: string[] = [];
			while (i < lines.length && /^\s*[-*+]\s+/.test(lines[i])) {
				items.push(`<li>${renderInline(lines[i].replace(/^\s*[-*+]\s+/, ''))}</li>`);
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

		// paragraph
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
		html.push(`<p>${renderInline(para.join(' '))}</p>`);
	}

	return html.join('\n');
}
