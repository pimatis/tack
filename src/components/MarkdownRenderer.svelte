<script lang="ts">
	import { renderMarkdown } from '$lib/markdown/render';

	type Props = {
		content: string;
		class?: string;
	};

	let { content, class: className = '' }: Props = $props();
	let html = $derived(renderMarkdown(content));
</script>

{#if html}
	<div class="prose prose-sm max-w-none prose-invert {className}">
		<!-- renderMarkdown escapes all user input; {@html} is safe here -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html html}
	</div>
{/if}
