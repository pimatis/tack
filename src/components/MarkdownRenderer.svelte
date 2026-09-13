<script lang="ts">
	import { mount, unmount, tick } from 'svelte';
	import { renderMarkdown } from '$lib/markdown/render';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';

	type Props = {
		content: string;
		class?: string;
		onToggleLine?: (line: number) => void;
		onOpenMention?: (href: string) => void;
		onMentionHover?: (href: string, anchor: HTMLElement) => void;
		onMentionLeave?: () => void;
		onOpenWiki?: (name: string) => void;
		onOpenTag?: (tag: string) => void;
		// right-click on a task-list row: line index + viewport coords
		onTodoMenu?: (line: number, x: number, y: number) => void;
		resolveAsset?: (rel: string) => string;
		// markdown line to scroll to and flash (block link navigation)
		highlight?: { line: number; nonce: number } | null;
	};

	let {
		content,
		class: className = '',
		onToggleLine,
		onOpenMention,
		onMentionHover,
		onMentionLeave,
		onOpenWiki,
		onOpenTag,
		onTodoMenu,
		resolveAsset,
		highlight
	}: Props = $props();
	let html = $derived(renderMarkdown(content, { resolveAsset, highlightLine: highlight?.line }));
	let container = $state<HTMLElement | null>(null);

	// scroll the marked block into view and flash it whenever a new
	// navigation request arrives (each request is a fresh highlight object)
	$effect(() => {
		if (!highlight) return;
		const root = container;
		if (!root) return;
		void tick().then(() => {
			const block = root.querySelector('[data-target-line]');
			if (!block) return;
			block.scrollIntoView({ block: 'center' });
			block.classList.add('note-line-flash');
			setTimeout(() => block.classList.remove('note-line-flash'), 1600);
		});
	});

	const COPY_ICON =
		'<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" d="M9 2a2 2 0 0 0-2 2v2h2V4h11v11h-2v2h2a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2zM4 7a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2z"/></svg>';
	const COPIED_ICON =
		'<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m3.535 6.381-4.95 4.95-2.12-2.121a1 1 0 0 0-1.415 1.414l2.758 2.758a1.1 1.1 0 0 0 1.556 0l5.586-5.586a1 1 0 0 0-1.415-1.415"/></svg>';

	// injected buttons and checkboxes are real elements; clicks are handled by delegation here
	async function handleClick(event: MouseEvent) {
		// task:/note: mention links navigate inside the app instead of the browser
		const mention = (event.target as HTMLElement).closest?.('a[href^="task:"], a[href^="note:"]');
		if (mention instanceof HTMLAnchorElement) {
			event.preventDefault();
			onOpenMention?.(mention.getAttribute('href') ?? '');
			return;
		}
		// [[wiki]] links resolve by note name in the host
		const wiki = (event.target as HTMLElement).closest?.('a[data-wiki]');
		if (wiki instanceof HTMLAnchorElement) {
			event.preventDefault();
			onOpenWiki?.(wiki.dataset.wiki ?? '');
			return;
		}
		// inline #tags filter the sidebar list
		const tag = (event.target as HTMLElement).closest?.('span[data-tag]');
		if (tag instanceof HTMLElement) {
			onOpenTag?.(tag.dataset.tag ?? '');
			return;
		}
		const button = (event.target as HTMLElement).closest?.('.copy-code-btn');
		if (!(button instanceof HTMLButtonElement)) return;
		const code = button.parentElement?.querySelector('code')?.textContent ?? '';
		try {
			await navigator.clipboard.writeText(code);
			button.innerHTML = COPIED_ICON;
			setTimeout(() => (button.innerHTML = COPY_ICON), 1500);
		} catch {
			// clipboard unavailable (permissions) - stay quiet
		}
	}

	// right-click on a task-list row: report the markdown line so the host can
	// offer conversions (checkbox → real task)
	function handleContextMenu(event: MouseEvent) {
		const row = (event.target as HTMLElement).closest?.('li');
		const todo = row?.querySelector('[data-todo]');
		if (!(todo instanceof HTMLElement)) return;
		event.preventDefault();
		onTodoMenu?.(Number(todo.dataset.todo), event.clientX, event.clientY);
	}

	// hover delegation for mention previews; the anchor element lets the
	// parent position the preview card next to the link
	function handleMouseOver(event: MouseEvent) {
		const mention = (event.target as HTMLElement).closest?.('a[href^="task:"], a[href^="note:"]');
		if (mention instanceof HTMLAnchorElement) {
			onMentionHover?.(mention.getAttribute('href') ?? '', mention);
		}
	}

	function handleMouseOut(event: MouseEvent) {
		const from = event.target as HTMLElement;
		const to = event.relatedTarget as HTMLElement | null;
		const mention = from.closest?.('a[href^="task:"], a[href^="note:"]');
		if (mention instanceof HTMLAnchorElement && !(to && mention.contains(to))) {
			onMentionLeave?.();
		}
	}

	// task checkboxes are placeholder spans in the html; mount the real ui
	// Checkbox component into each one so clicks write back to the markdown
	$effect(() => {
		void html;
		const root = container;
		if (!root) return;
		const mounted: ReturnType<typeof mount>[] = [];
		root.querySelectorAll<HTMLElement>('span[data-todo]').forEach((el) => {
			const line = Number(el.dataset.todo);
			const checked = el.dataset.checked === 'true';
			mounted.push(
				mount(Checkbox, {
					target: el,
					props: {
						checked,
						onclick: () => onToggleLine?.(line)
					}
				})
			);
		});
		return () => {
			for (const instance of mounted) unmount(instance);
		};
	});
</script>

{#if html}
	<!-- the injected copy buttons are real <button>s; the div is only a click relay -->
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
	<div
		bind:this={container}
		onclick={handleClick}
		oncontextmenu={handleContextMenu}
		onmouseover={handleMouseOver}
		onmouseout={handleMouseOut}
		class="prose prose-sm max-w-none prose-invert {className}"
	>
		<!-- renderMarkdown escapes all user input; {@html} is safe here -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html html}
	</div>
{/if}
