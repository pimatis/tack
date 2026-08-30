<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { SVGAttributes } from 'svelte/elements';

	let {
		class: className,
		role = 'status',
		'aria-label': ariaLabel = 'Loading',
		...restProps
	}: SVGAttributes<SVGSVGElement> = $props();
</script>

<svg
	{role}
	aria-label={ariaLabel}
	class={cn('size-4 animate-spin', className)}
	viewBox="0 0 24 24"
	fill="none"
	{...restProps}
>
	<path
		fill="none"
		stroke="currentColor"
		stroke-linecap="round"
		stroke-width="2"
		d="M12 3a9 9 0 1 0 9 9"
	/>
</svg>
