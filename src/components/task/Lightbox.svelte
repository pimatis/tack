<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';

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
						<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M20 14.5a1.5 1.5 0 0 1 1.5 1.5v4a2.5 2.5 0 0 1-2.5 2.5H5A2.5 2.5 0 0 1 2.5 20v-4a1.5 1.5 0 0 1 3 0v3.5h13V16a1.5 1.5 0 0 1 1.5-1.5m-8-13A1.5 1.5 0 0 1 13.5 3v9.036l1.682-1.682a1.5 1.5 0 0 1 2.121 2.12l-4.066 4.067a1.75 1.75 0 0 1-2.474 0l-4.066-4.066a1.5 1.5 0 0 1 2.121-2.121l1.682 1.682V3A1.5 1.5 0 0 1 12 1.5"
							/></svg
						>
					</button>
				{/if}
				<button
					class="flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20"
					onclick={onClose}
					aria-label="Close preview"
				>
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="m12 14.122 5.303 5.303a1.5 1.5 0 0 0 2.122-2.122L14.12 12l5.304-5.303a1.5 1.5 0 1 0-2.122-2.121L12 9.879 6.697 4.576a1.5 1.5 0 1 0-2.122 2.12L9.88 12l-5.304 5.304a1.5 1.5 0 1 0 2.122 2.12z"
						/></svg
					>
				</button>
			</div>
		</Dialog.Content>
	</Dialog.Root>
{/if}
