<script lang="ts">
	import { mount, unmount } from 'svelte';
	import { renderMarkdown } from '$lib/markdown/render';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';

	type Props = {
		content: string;
		class?: string;
		onToggleLine?: (line: number) => void;
	};

	let { content, class: className = '', onToggleLine }: Props = $props();
	let html = $derived(renderMarkdown(content));
	let container = $state<HTMLElement | null>(null);

	const COPY_ICON =
		'<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" d="M9 18q-.825 0-1.412-.587T7 16V4q0-.825.588-1.412T9 2h9q.825 0 1.413.588T20 4v12q0 .825-.587 1.413T18 18zm-4 4q-.825 0-1.412-.587T3 20V6h2v14h11v2z"/></svg>';
	const COPIED_ICON =
		'<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" d="m10.6 16.6l7.05-7.05l-1.4-1.4l-5.65 5.65l-2.85-2.85l-1.4 1.4zM12 22q-2.075 0-3.9-.788t-3.175-2.137T2.788 15.9T2 12t.788-3.9t2.137-3.175T8.1 2.788T12 2t3.9.788t3.175 2.137T21.213 8.1T22 12t-.788 3.9t-2.137 3.175t-3.175 2.138T12 22"/></svg>';

	// injected buttons and checkboxes are real elements; clicks are handled by delegation here
	async function handleClick(event: MouseEvent) {
		// task:/note: mention links and [[wiki]] links from old content are
		// dead ends; swallow the click instead of triggering scheme navigation
		const dead = (event.target as HTMLElement).closest?.(
			'a[href^="task:"], a[href^="note:"], a[data-wiki]'
		);
		if (dead instanceof HTMLAnchorElement) {
			event.preventDefault();
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
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		bind:this={container}
		onclick={handleClick}
		class="prose prose-sm max-w-none prose-invert {className}"
	>
		<!-- renderMarkdown escapes all user input; {@html} is safe here -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html html}
	</div>
{/if}
