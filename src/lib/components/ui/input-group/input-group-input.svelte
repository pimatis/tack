<script lang="ts">
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";
	import type { WithElementRef } from "$lib/utils.js";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		type = "text",
		value = $bindable(),
		...restProps
	}: WithElementRef<HTMLInputAttributes> & {
		type?: HTMLInputTypeAttribute;
		value?: string;
	} = $props();
</script>

<input
	bind:this={ref}
	bind:value
	data-slot="input-group-input"
	class={cn(
		"flex h-full w-full border-none bg-transparent px-3 py-2 text-sm shadow-none outline-none focus-visible:ring-0 disabled:cursor-not-allowed disabled:opacity-50",
		className
	)}
	{type}
	{...restProps}
/>
