<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button/index.js';

	let checked = $state(false);
	let installing = $state(false);
	let installingFailed = $state(false);
	let installError = $state('');

	async function checkInstalled() {
		try {
			checked = await invoke<boolean>('cli_installed');
		} catch {
			checked = false;
		}
	}

	async function handleInstall() {
		installing = true;
		installingFailed = false;
		installError = '';
		try {
			checked = await invoke<boolean>('install_cli');
		} catch (e) {
			console.error('install cli failed', e);
			installingFailed = true;
			checked = false;
			installError = typeof e === 'string' ? e : '';
		}
		installing = false;
	}

	onMount(() => {
		void checkInstalled();
		const onChanged = () => void checkInstalled();
		window.addEventListener('cli-path-changed', onChanged);
		return () => window.removeEventListener('cli-path-changed', onChanged);
	});
</script>

<div class="flex flex-wrap items-center justify-between gap-3">
	<div class="min-w-0">
		<p class="text-[13px] font-medium">Install CLI</p>
		<p class="text-xs text-muted-foreground">
			Add the <code class="font-mono text-foreground/80">tack</code> command to your PATH so you can use
			it from any terminal
		</p>
		{#if installingFailed}
			<p class="mt-1 text-xs text-destructive">
				{installError
					? installError
					: "Couldn't install the CLI. The tack command needs a writable bin directory."}
			</p>
		{/if}
	</div>

	<div class="flex shrink-0 items-center gap-2">
		{#if checked}
			<span
				class="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-500"
			>
				<svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"
					><path
						fill="currentColor"
						d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m3.535 6.381-4.95 4.95-2.12-2.121a1 1 0 0 0-1.415 1.414l2.758 2.758a1.1 1.1 0 0 0 1.556 0l5.586-5.586a1 1 0 0 0-1.415-1.415"
					/></svg
				>
				Installed
			</span>
		{:else}
			<Button
				variant="outline"
				size="sm"
				onclick={() => void handleInstall()}
				disabled={installing}
			>
				{installing ? 'installing…' : 'install path'}
			</Button>
		{/if}
	</div>
</div>
