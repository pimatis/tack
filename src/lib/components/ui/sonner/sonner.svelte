<script lang="ts">
	import { mode } from 'mode-watcher';
	import { Toaster as Sonner, type ToasterProps as SonnerProps } from 'svelte-sonner';
	import Icon from '$lib/components/Icon.svelte';
	import { Spinner } from '$lib/components/ui/spinner/index.js';

	let { ...restProps }: SonnerProps = $props();
</script>

<Sonner
	theme={mode.current}
	class="toaster group"
	style="--normal-bg: var(--color-popover); --normal-text: var(--color-popover-foreground); --normal-border: var(--color-border);"
	toastOptions={{
		classes: {
			toast: 'cn-toast'
		}
	}}
	{...restProps}
>
	{#snippet loadingIcon()}
		<Spinner />
	{/snippet}
	{#snippet successIcon()}
		<Icon icon="check-circle" class="size-4" />
	{/snippet}
	{#snippet errorIcon()}
		<Icon icon="close-circle" class="size-4" />
	{/snippet}
	{#snippet infoIcon()}
		<Icon icon="info-circle" class="size-4" />
	{/snippet}
	{#snippet warningIcon()}
		<Icon icon="alert-triangle" class="size-4" />
	{/snippet}
</Sonner>
