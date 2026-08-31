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
	data-slot="input-group-addon"
	class={cn('flex h-full items-center px-2.5 text-muted-foreground', className)}
	{...restProps}
>
	{@render children?.()}
</div>
