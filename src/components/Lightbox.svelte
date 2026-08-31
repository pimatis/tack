<script lang="ts">
	import { fade, scale } from 'svelte/transition';

	let {
		url,
		onClose
	}: {
		url: string | null;
		onClose: () => void;
	} = $props();

	let ref = $state<HTMLDivElement | null>(null);

	$effect(() => {
		if (!url) return;
		const onKeydown = (e: KeyboardEvent) => {
			if (e.key === 'Escape') {
				onClose();
				e.stopPropagation();
			}
		};
		const onPointerDown = (e: PointerEvent) => {
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
		class="fixed inset-0 z-[60] flex items-center justify-center bg-black/80 p-8"
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
		<button
			class="absolute top-4 right-4 flex size-8 items-center justify-center rounded-full bg-white/10 text-white transition-colors hover:bg-white/20"
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
