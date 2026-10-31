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
		'<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><g fill="currentColor"><path d="M15.2402 5.61426C17.9551 5.61426 19.3127 5.61431 20.1562 6.46094C20.9998 7.30786 21 8.67127 21 11.3975V16.2168C21 18.9429 20.9998 20.3064 20.1562 21.1533C19.3127 21.9998 17.955 22 15.2402 22H12.3604C9.64515 22 8.2869 22.0002 7.44336 21.1533C6.59984 20.3064 6.60059 18.9429 6.60059 16.2168V11.3975C6.60059 8.67128 6.59984 7.30786 7.44336 6.46094C8.28689 5.61402 9.64506 5.61426 12.3604 5.61426H15.2402Z"/><path d="M15.2402 2C16.7616 2.0001 18.0629 2.94764 18.5898 4.28711C17.7091 4.16822 16.6114 4.16887 15.335 4.16895H12.2656C10.9891 4.16887 9.8886 4.16821 9.00781 4.28711C8.06393 4.41454 7.15966 4.70266 6.42578 5.43945C5.69187 6.17631 5.40523 7.08449 5.27832 8.03223C5.15992 8.91654 5.16008 10.0211 5.16016 11.3027V16.3125C5.1601 17.374 5.16027 18.6502 5.22754 19.5596C3.92032 19.0177 3 17.7249 3 16.2168V10.3789C2.99999 8.60799 3.00049 7.20531 3.14746 6.10742C3.29878 4.97738 3.61751 4.06213 4.33594 3.34082C5.05433 2.61968 5.96542 2.29937 7.09082 2.14746C8.18439 1.99988 9.58168 1.99999 11.3457 2H15.2402Z"/></g></svg>';
	const COPIED_ICON =
		'<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path fill="currentColor" fill-rule="evenodd" d="M22 12C22 17.5228 17.5228 22 12 22C6.47715 22 2 17.5228 2 12C2 6.47715 6.47715 2 12 2C17.5228 2 22 6.47715 22 12ZM16.0303 8.96967C16.3232 9.26256 16.3232 9.73744 16.0303 10.0303L11.0303 15.0303C10.7374 15.3232 10.2626 15.3232 9.96967 15.0303L7.96967 13.0303C7.67678 12.7374 7.67678 12.2626 7.96967 11.9697C8.26256 11.6768 8.73744 11.6768 9.03033 11.9697L10.5 13.4393L12.7348 11.2045L14.9697 8.96967C15.2626 8.67678 15.7374 8.67678 16.0303 8.96967Z" clip-rule="evenodd"/></svg>';

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
