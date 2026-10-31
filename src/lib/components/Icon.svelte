<script lang="ts">
	import { ICONS, iconName, loadIconBodies, peekIconBody, type IconName } from '$lib/icons';

	let {
		icon,
		size = 14,
		class: className
	}: { icon: IconName; size?: number; class?: string } = $props();

	let body = $state<string | null>(null);

	$effect(() => {
		const name = iconName(ICONS[icon]);
		const cached = peekIconBody(name);
		if (cached !== null) {
			body = cached;
			return;
		}
		body = null;
		let cancelled = false;
		void loadIconBodies([name])
			.then((bodies) => {
				if (!cancelled) body = bodies[name] ?? null;
			})
			.catch(() => {});
		return () => {
			cancelled = true;
		};
	});
</script>

{#if body}
	<svg
		viewBox="0 0 24 24"
		width={size}
		height={size}
		fill="currentColor"
		class={className}
		aria-hidden="true"
	>
		<!-- sanitized by the icon loader before it reaches the dom -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html body}
	</svg>
{/if}
