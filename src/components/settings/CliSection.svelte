<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button/index.js';
	import { isTauri } from '$lib/db/client';
	import Icon from '$lib/components/Icon.svelte';

	const browser = !isTauri();

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
		if (browser) return;
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
			{#if browser}
				The <code class="font-mono text-foreground/80">tack</code> command is installed from the desktop
				app.
			{:else}
				Add the <code class="font-mono text-foreground/80">tack</code> command to your PATH so you can
				use it from any terminal
			{/if}
		</p>
		{#if installingFailed}
			<p class="mt-1 text-xs text-destructive">
				{installError
					? installError
					: "Couldn't install the CLI. The tack command needs a writable bin directory."}
			</p>
		{/if}
	</div>

	{#if !browser}
		<div class="flex shrink-0 items-center gap-2">
			{#if checked}
				<span
					class="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-500"
				>
					<Icon icon="check-circle" size={15} />
					Installed
				</span>
			{:else}
				<Button
					variant="outline"
					size="sm"
					onclick={() => void handleInstall()}
					disabled={installing}
				>
					{installing ? 'Installing…' : 'Install path'}
				</Button>
			{/if}
		</div>
	{/if}
</div>
