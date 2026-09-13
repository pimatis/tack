<script lang="ts">
	import { fade, scale } from 'svelte/transition';

	let {
		url,
		onClose,
		onDownload
	}: {
		url: string | null;
		onClose: () => void;
		onDownload?: () => void;
	} = $props();

	let ref = $state<HTMLDivElement | null>(null);

	$effect(() => {
		if (!url) return;
		const onKeydown = (e: KeyboardEvent) => {
			if (!document.hasFocus()) return;
			if (e.key === 'Escape') {
				onClose();
				e.stopPropagation();
			}
		};
		const onPointerDown = (e: PointerEvent) => {
			if (!document.hasFocus()) return;
			if (!ref) return;
			if (ref.contains(e.target as Node)) {
				e.stopPropagation();
				onClose();
			}
		};
		document.addEventListener('keydown', onKeydown, { capture: true });
		document.addEventListener('pointerdown', onPointerDown, { capture: true });
		return () => {
			document.removeEventListener('keydown', onKeydown, { capture: true });
			document.removeEventListener('pointerdown', onPointerDown, { capture: true });
		};
	});
</script>

{#if url}
	<div
		bind:this={ref}
		class="fixed inset-0 z-[60] flex items-center justify-center bg-black/80 p-3 sm:p-8"
		onclick={onClose}
		onpointerdowncapture={(e) => {
			e.stopPropagation();
			onClose();
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') {
				onClose();
				e.stopPropagation();
			}
		}}
		role="button"
		tabindex="-1"
		transition:fade={{ duration: 150 }}
	>
		<img
			src={url}
			alt="preview"
			class="max-h-full max-w-full rounded-lg object-contain shadow-2xl"
			transition:scale={{ duration: 150, start: 0.95 }}
		/>
		{#if onDownload}
			<button
				class="absolute top-2 right-12 flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20 sm:top-4 sm:right-14"
				onclick={(e) => {
					e.stopPropagation();
					onDownload();
				}}
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
			class="absolute top-2 right-2 flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20 sm:top-4 sm:right-4"
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
{/if}
