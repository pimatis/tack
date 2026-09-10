<script lang="ts">
	import { mount, unmount } from 'svelte';
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
		resolveAsset?: (rel: string) => string;
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
		resolveAsset
	}: Props = $props();
	let html = $derived(renderMarkdown(content, { resolveAsset }));
	let container = $state<HTMLElement | null>(null);

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
		onmouseover={handleMouseOver}
		onmouseout={handleMouseOut}
		class="prose prose-sm max-w-none prose-invert {className}"
	>
		<!-- renderMarkdown escapes all user input; {@html} is safe here -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html html}
	</div>
{/if}
