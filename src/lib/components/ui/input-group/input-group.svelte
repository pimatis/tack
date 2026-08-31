<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		ref?: HTMLDivElement | null;
		children: Snippet;
	} = $props();
</script>

<div
	bind:this={ref}
	data-slot="input-group"
	class={cn(
		'group/input-group flex h-10 items-center rounded-md border border-input bg-background text-sm shadow-xs',
		className
	)}
	{...restProps}
>
	{@render children?.()}
</div>
