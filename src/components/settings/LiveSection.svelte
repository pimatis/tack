<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { isTauri } from '$lib/db/client';
	import { invoke } from '@tauri-apps/api/core';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import {
		getLiveStatus,
		getLivePresence,
		type LiveStatus,
		type PresenceClient
	} from '$lib/live/live.service';
	import type { Settings } from '$lib/types/settings';

	let {
		settings,
		update
	}: {
		settings: Settings;
		update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
	} = $props();

	const browser = !isTauri();

	let status = $state<LiveStatus | null>(null);
	let presence = $state<PresenceClient[]>([]);
	let error = $state('');
	let copied = $state('');
	let busy = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	// shared password (set / change / remove)
	let pwDialogOpen = $state(false);
	let pw = $state('');
	let pwConfirm = $state('');
	let pwError = $state('');
	let pwBusy = $state(false);

	const passwordSet = $derived(settings.livePasswordHash !== '');

	function openPasswordDialog() {
		pw = '';
		pwConfirm = '';
		pwError = '';
		pwDialogOpen = true;
	}

	async function savePassword() {
		if (pw.length < 8) {
			pwError = 'use at least 8 characters';
			return;
		}
		if (pw !== pwConfirm) {
			pwError = 'passwords do not match';
			return;
		}
		pwBusy = true;
		pwError = '';
		try {
			// hashing happens in rust (pbkdf2); the plaintext never touches disk
			const { hash, salt } = await invoke<{ hash: string; salt: string }>('hash_live_password', {
				password: pw
			});
			update('livePasswordHash', hash);
			update('livePasswordSalt', salt);
			pwDialogOpen = false;
		} catch (e) {
			pwError = String(e);
		}
		pwBusy = false;
	}

	function removePassword() {
		update('livePasswordHash', '');
		update('livePasswordSalt', '');
	}

	async function refresh() {
		status = await getLiveStatus();
		presence = await getLivePresence();
	}

	async function handleToggle(enabled: boolean) {
		busy = true;
		error = '';
		try {
			update('liveEnabled', enabled);
			// give the live manager a moment to start or stop the server
			await new Promise((resolve) => setTimeout(resolve, 400));
			await refresh();
			if (enabled && status) void openInBrowser();
		} catch {
			error = 'the live server could not be reached';
		}
		busy = false;
	}

	async function handlePortChange(value: string) {
		const port = Number(value);
		if (!Number.isInteger(port) || port < 1024 || port > 65535) return;
		error = '';
		update('livePort', port);
		// the manager restarts the server on the new port
		await new Promise((resolve) => setTimeout(resolve, 400));
		await refresh();
	}

	async function openInBrowser() {
		if (!status) return;
		if (browser) {
			window.open(status.url, '_blank');
			return;
		}
		try {
			const { openUrl } = await import('@tauri-apps/plugin-opener');
			await openUrl(status.url);
		} catch {
			window.open(status.url, '_blank');
		}
	}

	async function copyText(text: string, target: 'url' | 'stream') {
		try {
			await navigator.clipboard.writeText(text);
		} catch {
			const el = document.createElement('textarea');
			el.value = text;
			document.body.appendChild(el);
			el.select();
			document.execCommand('copy');
			el.remove();
		}
		copied = target;
		if (copyTimer) clearTimeout(copyTimer);
		copyTimer = setTimeout(() => (copied = ''), 3000);
	}

	onMount(() => {
		void refresh();
		const interval = window.setInterval(() => void refresh(), 2000);
		const onStatus = () => void refresh();
		const onError = (event: Event) => {
			error = (event as CustomEvent<string>).detail;
		};
		window.addEventListener('live-status-changed', onStatus);
		window.addEventListener('live-error-changed', onError);
		return () => {
			window.clearInterval(interval);
			window.removeEventListener('live-status-changed', onStatus);
			window.removeEventListener('live-error-changed', onError);
			if (copyTimer) clearTimeout(copyTimer);
		};
	});
</script>

{#if browser}
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Live server</p>
			<p class="text-xs text-muted-foreground">
				You're viewing tack through the live server running on your computer
			</p>
		</div>
		<span
			class="inline-flex shrink-0 items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-500"
		>
			<span class="size-1.5 rounded-full bg-emerald-500"></span>
			Live
		</span>
	</div>
{:else}
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Live server</p>
			<p class="text-xs text-muted-foreground">
				Share your workspace in a browser on this device or your local network
			</p>
		</div>
		<Switch
			checked={settings.liveEnabled}
			onCheckedChange={(v) => void handleToggle(v)}
			disabled={busy}
		/>
	</div>

	{#if settings.liveEnabled}
		<Separator />
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div>
				<p class="text-[13px] font-medium">Port</p>
				<p class="text-xs text-muted-foreground">Where the server listens on this device</p>
			</div>
			<Input
				type="number"
				min={1024}
				max={65535}
				value={settings.livePort}
				oninput={(e) => void handlePortChange((e.currentTarget as HTMLInputElement).value)}
				class="w-28 text-right"
			/>
		</div>

		<Separator />
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Password</p>
				<p class="text-xs text-muted-foreground">
					{#if passwordSet}
						Visitors must enter the password to open the share
					{:else}
						Optional: require a password to open the share
					{/if}
				</p>
			</div>
			<div class="flex shrink-0 items-center gap-2">
				{#if passwordSet}
					<Button variant="outline" size="sm" onclick={openPasswordDialog}>Change</Button>
					<Button variant="ghost" size="sm" onclick={removePassword}>Remove</Button>
				{:else}
					<Button variant="outline" size="sm" onclick={openPasswordDialog}>Set password</Button>
				{/if}
			</div>
		</div>

		<Separator />
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Status</p>
				{#if status}
					<button
						type="button"
						onclick={() => {
							if (status) void copyText(status.url, 'url');
						}}
						title="Copy url"
						aria-label="Copy url"
						class="mt-1 flex max-w-full cursor-pointer items-center gap-1.5 rounded-md font-mono text-xs text-muted-foreground transition-colors hover:text-foreground"
					>
						<span class="size-1.5 shrink-0 rounded-full bg-emerald-500"></span>
						<span class="truncate">{copied === 'url' ? 'Copied!' : status.url}</span>
					</button>
				{:else}
					<p class="text-xs text-muted-foreground">
						{error ? 'could not start' : 'starting…'}
					</p>
				{/if}
			</div>
			<div class="flex shrink-0 items-center gap-2">
				{#if status}
					<Button
						variant="ghost"
						size="icon-sm"
						onclick={() => void handleToggle(false)}
						aria-label="Stop live server"
					>
						<svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor"
							><path
								fill="currentColor"
								d="M12 2c5.523 0 10 4.477 10 10s-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2m2 6h-4a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h4a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2"
							/></svg
						>
					</Button>
					<Button variant="outline" size="sm" onclick={() => void openInBrowser()}>
						Open in browser
					</Button>
				{/if}
			</div>
		</div>

		<Separator />
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Connected clients</p>
				<p class="text-xs text-muted-foreground">
					{presence.length === 0
						? 'No one else is connected right now'
						: `${presence.length} connected`}
				</p>
			</div>
			{#if presence.length > 0}
				<div class="flex max-w-[60%] flex-wrap justify-end gap-1.5">
					{#each presence as client (client.id)}
						<span class="rounded-full bg-muted/50 px-2 py-0.5 text-[11px] text-muted-foreground">
							{client.name}
						</span>
					{/each}
				</div>
			{/if}
		</div>

		<Separator />
		<div class="flex flex-wrap items-center justify-between gap-3">
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Event stream</p>
				<p class="text-xs text-muted-foreground">
					Real-time db change feed for agents (Server-Sent Events)
				</p>
			</div>
			{#if status}
				<button
					type="button"
					onclick={() => {
						if (status) void copyText(`${status.url}/api/events/stream`, 'stream');
					}}
					title="Copy event stream url"
					aria-label="Copy event stream url"
					class="flex max-w-full cursor-pointer items-center gap-1.5 rounded-md font-mono text-xs text-muted-foreground transition-colors hover:text-foreground"
				>
					<span class="size-1.5 shrink-0 rounded-full bg-emerald-500"></span>
					<span class="truncate"
						>{copied === 'stream' ? 'Copied!' : `${status.url}/api/events/stream`}</span
					>
				</button>
			{:else}
				<p class="text-xs text-muted-foreground">available while the server runs</p>
			{/if}
		</div>

		{#if error}
			<p class="text-xs text-destructive">{error}</p>
		{:else}
			<p class="text-xs text-muted-foreground">
				Your data stays on this device. While the server is on, any device on your local network can
				open the address above{#if passwordSet}, but they need the password to see anything{/if}.
				Agents can watch changes in real time with
				<code class="font-mono text-foreground/80">tack live watch</code>
				{#if status}or
					<code class="font-mono text-foreground/80">curl -N {status.url}/api/events/stream</code
					>{/if}. You can also toggle it from the terminal with
				<code class="font-mono text-foreground/80">tack live on</code> or
				<code class="font-mono text-foreground/80">tack live off</code>
			</p>
		{/if}
	{/if}
{/if}

{#if pwDialogOpen}
	<Dialog.Root bind:open={pwDialogOpen}>
		<Dialog.Content class="w-[calc(100vw-2rem)] max-w-sm gap-4 p-4 sm:p-6">
			<Dialog.Header class="gap-1.5">
				<Dialog.Title>{passwordSet ? 'Change password' : 'Set password'}</Dialog.Title>
				<Dialog.Description>
					Visitors opening the live share must enter this password.
				</Dialog.Description>
			</Dialog.Header>
			<form
				class="flex flex-col gap-3"
				onsubmit={(e) => {
					e.preventDefault();
					void savePassword();
				}}
			>
				<Input
					type="password"
					placeholder="Password (min 8 characters)"
					autocomplete="new-password"
					bind:value={pw}
					disabled={pwBusy}
				/>
				<Input
					type="password"
					placeholder="Repeat password"
					autocomplete="new-password"
					bind:value={pwConfirm}
					disabled={pwBusy}
				/>
				{#if pwError}
					<p class="text-xs text-destructive">{pwError}</p>
				{/if}
				<Dialog.Footer>
					<Button type="button" variant="ghost" onclick={() => (pwDialogOpen = false)}>
						Cancel
					</Button>
					<Button type="submit" disabled={pwBusy || pw.length < 8}>Save</Button>
				</Dialog.Footer>
			</form>
		</Dialog.Content>
	</Dialog.Root>
{/if}
