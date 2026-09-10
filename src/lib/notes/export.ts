import { invoke } from '@tauri-apps/api/core';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { renderMarkdown } from '$lib/markdown/render';

// export a note as markdown copy or a self-contained html document

const HTML_SHELL = (title: string, body: string) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${title}</title>
<style>
  :root { color-scheme: light dark; }
  body { max-width: 46rem; margin: 0 auto; padding: 3rem 1.5rem;
         font: 15px/1.65 -apple-system, "Segoe UI", Roboto, sans-serif;
         color: #1a1a1a; background: #fff; }
  pre { background: #f6f6f6; border: 1px solid #e5e5e5; border-radius: 8px;
        padding: 0.9rem 1rem; overflow-x: auto; font-size: 13px; }
  code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.9em; }
  blockquote { margin: 0; padding: 0.2rem 1rem; border-left: 3px solid #d4d4d4; color: #555; }
  table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
  th, td { border: 1px solid #e5e5e5; padding: 0.45rem 0.7rem; text-align: left; }
  th { background: #fafafa; }
  img { max-width: 100%; }
  h1, h2, h3 { line-height: 1.3; }
  @media (prefers-color-scheme: dark) {
    body { color: #e5e5e5; background: #17171a; }
    pre { background: #202024; border-color: #2e2e33; }
    blockquote { border-color: #3a3a3f; color: #b3b3b8; }
    th { background: #202024; } th, td { border-color: #2e2e33; }
  }
</style>
</head>
<body>${body}</body>
</html>`;

export type ExportFormat = 'md' | 'html';

export async function exportNote(
	notePath: string,
	folder: string,
	format: ExportFormat
): Promise<string | null> {
	const name = notePath.split('/').pop()?.replace(/\.md$/i, '') ?? 'note';
	const content = await invoke<string>('read_file', { path: notePath });
	const target = await saveDialog({
		defaultPath: `${name}.${format}`,
		filters:
			format === 'md'
				? [{ name: 'Markdown', extensions: ['md'] }]
				: [{ name: 'HTML', extensions: ['html'] }]
	});
	if (typeof target !== 'string') return null;
	if (format === 'md') {
		await invoke('write_file', { path: target, content });
	} else {
		// images resolve to absolute file paths so the export opens anywhere
		const body = renderMarkdown(content, {
			resolveAsset: (rel) => {
				const abs = rel.startsWith('.tack/')
					? `${folder}/${rel}`
					: rel.startsWith('/')
						? rel
						: `${folder}/${rel}`;
				return encodeURI(`file://${abs}`);
			}
		});
		await invoke('write_file', { path: target, content: HTML_SHELL(name, body) });
	}
	return target;
}
