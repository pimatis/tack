<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import { checkForUpdate, downloadAndInstall, relaunchApp } from '$lib/updater/update.service';
	import type { Update } from '$lib/updater/update.service';

	let { appVersion = '' }: { appVersion?: string } = $props();

	type CheckState =
		| { status: 'idle' }
		| { status: 'checking' }
		| { status: 'up-to-date' }
		| { status: 'available'; update: Update }
		| { status: 'downloading'; progress: number }
		| { status: 'installing' }
		| { status: 'error'; message: string };

	let checkState = $state<CheckState>({ status: 'idle' });
	let pendingUpdate = $state<Update | null>(null);
	let progressPct = $state(0);

	async function handleCheck() {
		checkState = { status: 'checking' };
		try {
			const update = await checkForUpdate();
			pendingUpdate = update;
			if (update) {
				checkState = { status: 'available', update };
			} else {
				checkState = { status: 'up-to-date' };
			}
		} catch {
			checkState = {
				status: 'error',
				message: 'Could not reach the update server. Check your connection and try again.'
			};
		}
	}

	async function handleDownload() {
		if (!pendingUpdate) return;
		checkState = { status: 'downloading', progress: 0 };
		try {
			await downloadAndInstall(pendingUpdate, (fraction) => {
				progressPct = Math.round(fraction * 100);
				checkState = { status: 'downloading', progress: fraction };
			});
			checkState = { status: 'installing' };
			await relaunchApp();
		} catch {
			checkState = {
				status: 'error',
				message: 'The update failed to download. Please try again.'
			};
		}
	}
</script>

<!-- version info -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div>
		<p class="text-[13px] font-medium">Version</p>
		<p class="text-xs text-muted-foreground">
			{appVersion || 'Unknown'}
		</p>
	</div>
	{#if checkState.status === 'idle'}
		<Button variant="outline" size="sm" onclick={() => void handleCheck()}>
			Check for updates
		</Button>
	{/if}
</div>

{#if checkState.status !== 'idle'}
	<div class="rounded-lg border border-border bg-muted/20 px-3 py-2.5">
		{#if checkState.status === 'checking'}
			<div class="flex items-center gap-2 text-[13px] text-muted-foreground">
				<Spinner class="size-3.5" />
				<span>Checking for updates...</span>
			</div>
		{:else if checkState.status === 'up-to-date'}
			<div class="flex flex-wrap items-center justify-between gap-2">
				<div class="flex items-center gap-2 text-[13px] text-muted-foreground">
					<svg
						width="15"
						height="15"
						viewBox="0 0 24 24"
						fill="currentColor"
						class="text-emerald-500"
						><path
							fill="currentColor"
							d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m3.535 6.381-4.95 4.95-2.12-2.121a1 1 0 0 0-1.415 1.414l2.758 2.758a1.1 1.1 0 0 0 1.556 0l5.586-5.586a1 1 0 0 0-1.415-1.415"
						/></svg
					>
					<span>You're on the latest version</span>
				</div>
				<Button variant="ghost" size="sm" class="h-7 text-xs" onclick={() => void handleCheck()}>
					Check again
				</Button>
			</div>
		{:else if checkState.status === 'available'}
			<div class="flex flex-wrap items-center justify-between gap-2">
				<div>
					<p class="text-[13px] font-medium">
						Version {checkState.update.version} is available
					</p>
					<p class="text-xs text-muted-foreground">
						This usually takes less than a minute to install.
					</p>
				</div>
				<div class="flex shrink-0 items-center gap-2">
					<Button
						variant="ghost"
						size="sm"
						class="h-7 text-xs"
						onclick={() => (checkState = { status: 'idle' })}
					>
						Dismiss
					</Button>
					<Button size="sm" onclick={() => void handleDownload()}>Download &amp; install</Button>
				</div>
			</div>
		{:else if checkState.status === 'downloading'}
			<div class="space-y-2">
				<div class="flex items-center justify-between text-[13px]">
					<span class="text-muted-foreground">Downloading update...</span>
					<span class="text-muted-foreground tabular-nums">{progressPct}%</span>
				</div>
				<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
					<div
						class="h-full rounded-full bg-primary transition-[width] duration-200"
						style="width: {progressPct}%"
					></div>
				</div>
			</div>
		{:else if checkState.status === 'installing'}
			<div class="flex items-center gap-2 text-[13px] text-muted-foreground">
				<Spinner class="size-3.5" />
				<span>Installing update. Restarting Tack...</span>
			</div>
		{:else if checkState.status === 'error'}
			<div class="flex flex-wrap items-center justify-between gap-2">
				<div class="flex items-center gap-2 text-[13px] text-destructive">
					<svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"
						><path
							fill="currentColor"
							d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m0 13a1 1 0 1 0 0 2 1 1 0 0 0 0-2m0-9a1 1 0 0 0-.993.883L11 7v6a1 1 0 0 0 1.993.117L13 13V7a1 1 0 0 0-1-1"
						/></svg
					>
					<span>{checkState.message}</span>
				</div>
				<Button variant="ghost" size="sm" class="h-7 text-xs" onclick={() => void handleCheck()}>
					Try again
				</Button>
			</div>
		{/if}
	</div>
{/if}
