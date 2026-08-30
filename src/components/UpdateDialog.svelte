<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import {
		checkForUpdate,
		downloadAndInstall,
		relaunchApp,
		getAppVersion
	} from '$lib/updater/update.service';
	import type { Update, UpdateState } from '$lib/updater/update.service';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	let uiState: UpdateState = $state({ status: 'idle' });
	let currentVersion = $state('');
	let pendingUpdate: Update | null = $state(null);
	let progressPct = $state(0);

	async function startCheck() {
		uiState = { status: 'checking' };
		try {
			const update = await checkForUpdate();
			pendingUpdate = update;
			if (update) {
				uiState = { status: 'available', update };
			} else {
				uiState = { status: 'error', message: "You're on the latest version already." };
			}
		} catch (e) {
			uiState = {
				status: 'error',
				message: 'Could not reach the update server. Check your connection and try again.'
			};
		}
	}

	async function startDownload() {
		if (!pendingUpdate) return;
		uiState = { status: 'downloading', progress: 0, contentLength: null };
		try {
			await downloadAndInstall(pendingUpdate, (fraction) => {
				progressPct = Math.round(fraction * 100);
				uiState = { status: 'downloading', progress: fraction, contentLength: null };
			});
			// installation finished - restart automatically so the new version takes effect
			uiState = { status: 'installing' };
			await relaunchApp();
		} catch (e) {
			uiState = {
				status: 'error',
				message: 'The update failed to download. Please try again.'
			};
		}
	}

	async function restartNow() {
		try {
			await relaunchApp();
		} catch (e) {
			uiState = {
				status: 'error',
				message: 'The app could not restart automatically. Please close and reopen Tack manually.'
			};
		}
	}

	$effect(() => {
		if (open) {
			void getAppVersion().then((v) => (currentVersion = v));
			if (uiState.status === 'idle') {
				void startCheck();
			}
		}
	});
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="max-w-md gap-5 p-6">
		<Dialog.Header class="gap-2">
			<Dialog.Title class="text-base font-semibold tracking-tight">
				{#if uiState.status === 'checking'}
					Checking for updates
				{:else if uiState.status === 'available'}
					Update available
				{:else if uiState.status === 'downloading'}
					Downloading update
				{:else if uiState.status === 'installing'}
					Restarting
				{:else if uiState.status === 'installed'}
					Update installed
				{:else if uiState.status === 'error'}
					Update check
				{:else}
					Software update
				{/if}
			</Dialog.Title>
			<Dialog.Description class="text-[13px] leading-relaxed text-muted-foreground">
				{#if uiState.status === 'checking'}
					Looking for a newer version of Tack…
				{:else if uiState.status === 'available' && uiState.update}
					version {uiState.update.version} is ready to install. This usually takes less than a minute.
				{:else if uiState.status === 'downloading'}
					Fetching version {pendingUpdate?.version ?? ''}…
				{:else if uiState.status === 'installing'}
					The new version is installed. Restarting Tack…
				{:else if uiState.status === 'installed'}
					Tack has been updated to version {uiState.version}. Restart the app to start using it.
				{:else if uiState.status === 'error'}
					{uiState.message}
				{:else}
					You are on version {currentVersion || '0.1.0'}.
				{/if}
			</Dialog.Description>
		</Dialog.Header>

		{#if uiState.status === 'downloading'}
			<div class="space-y-1.5">
				<div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
					<div
						class="h-full rounded-full bg-primary transition-[width] duration-200"
						style="width: {progressPct}%"
					></div>
				</div>
				<p class="text-right text-[11px] text-muted-foreground">{progressPct}%</p>
			</div>
		{/if}

		<Dialog.Footer class="mt-1">
			{#if uiState.status === 'checking'}
				<div class="flex items-center gap-2 text-[13px] text-muted-foreground">
					<Spinner class="size-3.5" />
					<span>Checking for updates…</span>
				</div>
			{:else if uiState.status === 'available'}
				<Button variant="outline" size="sm" onclick={() => (open = false)}>not now</Button>
				<Button size="sm" onclick={() => void startDownload()}>update now</Button>
			{:else if uiState.status === 'downloading' || uiState.status === 'installing'}
				<div class="flex items-center gap-2 text-[13px] text-muted-foreground">
					<Spinner class="size-3.5" />
					<span>
						{#if uiState.status === 'downloading'}
							Downloading update…
						{:else}
							Restarting…
						{/if}
					</span>
				</div>
			{:else if uiState.status === 'installed'}
				<Button variant="outline" size="sm" onclick={() => (open = false)}>later</Button>
				<Button size="sm" onclick={() => void restartNow()}>restart now</Button>
			{:else if uiState.status === 'error'}
				<Button variant="outline" size="sm" onclick={() => (open = false)}>close</Button>
			{/if}
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
