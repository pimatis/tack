<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import * as InputGroup from '$lib/components/ui/input-group/index.js';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		...restProps
	}: CommandPrimitive.InputProps = $props();
</script>

<div data-slot="command-input-wrapper" class="p-1 pb-0">
	<InputGroup.Root
		class="h-8! rounded-lg! border-input/30 bg-input/30 shadow-none! *:data-[slot=input-group-addon]:pl-2!"
	>
		<CommandPrimitive.Input
			{value}
			data-slot="command-input"
			class={cn(
				'w-full text-sm outline-hidden disabled:cursor-not-allowed disabled:opacity-50',
				className
			)}
			{...restProps}
		>
			{#snippet child({ props })}
				<InputGroup.Input {...props} bind:value bind:ref />
			{/snippet}
		</CommandPrimitive.Input>
		<InputGroup.Addon>
			<svg
				class="size-4 shrink-0 opacity-50"
				width="24"
				height="24"
				viewBox="0 0 24 24"
				fill="none"
			>
				<path
					fill="none"
					stroke="currentColor"
					stroke-linecap="round"
					stroke-width="2"
					d="m15.879 15.879l4.242 4.242M18 10.5a7.5 7.5 0 1 1-15 0a7.5 7.5 0 0 1 15 0Z"
				/>
			</svg>
		</InputGroup.Addon>
	</InputGroup.Root>
</div>
