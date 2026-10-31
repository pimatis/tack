<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import Icon from '$lib/components/Icon.svelte';

	let {
		url,
		onClose,
		onDownload
	}: {
		url: string | null;
		onClose: () => void;
		onDownload?: () => void;
	} = $props();
</script>

{#if url}
	<!-- a real dialog layer: while a sheet/dialog is open bits-ui disables
		pointer-events outside its content, so an inline fixed overlay would be
		unclickable and taps would fall through to the panel behind it -->
	<Dialog.Root open onOpenChange={(v) => (!v ? onClose() : undefined)}>
		<Dialog.Overlay class="z-[60] bg-black/80" />
		<Dialog.Content
			class="z-[60] w-auto max-w-none border-0 bg-transparent p-2 shadow-none"
			showCloseButton={false}
		>
			<Dialog.Title class="sr-only">Attachment preview</Dialog.Title>
			<img src={url} alt="preview" class="max-h-[85vh] max-w-full rounded-lg object-contain" />
			<div class="absolute top-4 right-4 flex items-center gap-2">
				{#if onDownload}
					<button
						class="flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20"
						onclick={() => onDownload()}
						aria-label="Download"
					>
						<Icon icon="download" size={16} />
					</button>
				{/if}
				<button
					class="flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20"
					onclick={onClose}
					aria-label="Close preview"
				>
					<Icon icon="close" size={16} />
				</button>
			</div>
		</Dialog.Content>
	</Dialog.Root>
{/if}
