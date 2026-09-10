<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { notesState } from '$lib/notes/notesState.svelte';
</script>

<Dialog.Root open={notesState.nameWarning !== null}>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">Name too long</Dialog.Title>
		<div class="flex flex-col items-start gap-3 px-4 py-5 sm:px-5">
			<div class="flex items-center gap-2.5">
				<svg class="shrink-0 text-amber-500" width="20" height="20" viewBox="0 0 24 24" fill="none"
					><path
						fill="currentColor"
						d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m-.01 8H11a1 1 0 0 0-.117 1.993L11 12v4.99c0 .52.394.95.9 1.004l.11.006h.49a1 1 0 0 0 .596-1.803L13 16.134V11.01c0-.52-.394-.95-.9-1.004zM12 7a1 1 0 1 0 0 2 1 1 0 0 0 0-2"
					/></svg
				>
				<span class="text-[14px] font-semibold tracking-tight text-foreground">Name too long</span>
			</div>
			<p class="text-[12px] leading-relaxed text-muted-foreground">
				{notesState.nameWarning}
			</p>
			<Button
				size="sm"
				class="ml-auto"
				onclick={() => {
					notesState.nameWarning = null;
				}}
			>
				OK
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
