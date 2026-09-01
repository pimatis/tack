<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { isTauri } from '$lib/db/client';
	import { getLiveStatus, type LiveStatus } from '$lib/live/live.service';
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
	let error = $state('');
	let copied = $state(false);
	let busy = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	async function refresh() {
		status = await getLiveStatus();
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

	async function copyUrl() {
		if (!status) return;
		try {
			await navigator.clipboard.writeText(status.url);
		} catch {
			const el = document.createElement('textarea');
			el.value = status.url;
			document.body.appendChild(el);
			el.select();
			document.execCommand('copy');
			el.remove();
		}
		copied = true;
		if (copyTimer) clearTimeout(copyTimer);
		copyTimer = setTimeout(() => (copied = false), 3000);
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
	<div class="flex items-center justify-between gap-4">
		<div class="flex min-w-0 items-start gap-3">
			<div
				class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-muted/30"
			>
				<svg
					width="17"
					height="17"
					viewBox="0 0 24 24"
					fill="currentColor"
					class="text-muted-foreground"
					><path
						fill="currentColor"
						d="M12 8.5a1.5 1.5 0 0 1 1.5 1.5v8.5H15a1.5 1.5 0 0 1 0 3H9a1.5 1.5 0 0 1 0-3h1.5V10A1.5 1.5 0 0 1 12 8.5M7.404 3.282a1.5 1.5 0 0 1 0 2.122A6.475 6.475 0 0 0 5.5 10c0 1.795.726 3.418 1.904 4.596a1.5 1.5 0 1 1-2.122 2.122A9.475 9.475 0 0 1 2.5 10c0-2.623 1.065-5 2.782-6.718a1.5 1.5 0 0 1 2.122 0m11.314 0A9.475 9.475 0 0 1 21.5 10c0 2.623-1.065 5-2.782 6.718a1.5 1.5 0 1 1-2.122-2.122A6.475 6.475 0 0 0 18.5 10a6.475 6.475 0 0 0-1.904-4.596 1.5 1.5 0 1 1 2.122-2.122m-8.486 2.829a1.5 1.5 0 0 1 .103 2.007l-.103.114A2.488 2.488 0 0 0 9.5 10c0 .392.09.76.248 1.088l.086.16a1.5 1.5 0 1 1-2.597 1.503A5.482 5.482 0 0 1 6.5 10c0-1.518.617-2.895 1.61-3.89a1.5 1.5 0 0 1 2.122 0Zm5.657 0A5.488 5.488 0 0 1 17.5 10c0 1-.268 1.94-.737 2.751a1.5 1.5 0 0 1-2.663-1.374l.066-.128A2.48 2.48 0 0 0 14.5 10c0-.622-.225-1.188-.601-1.627l-.131-.14a1.5 1.5 0 1 1 2.121-2.122"
					/></svg
				>
			</div>
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Live server</p>
				<p class="text-xs text-muted-foreground">
					You're viewing tack through the live server running on your computer
				</p>
			</div>
		</div>
		<span
			class="inline-flex shrink-0 items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-500"
		>
			<span class="size-1.5 rounded-full bg-emerald-500"></span>
			Live
		</span>
	</div>
{:else}
	<div class="flex items-center justify-between gap-4">
		<div class="flex min-w-0 items-start gap-3">
			<div
				class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-muted/30"
			>
				<svg
					width="17"
					height="17"
					viewBox="0 0 24 24"
					fill="currentColor"
					class="text-muted-foreground"
					><path
						fill="currentColor"
						d="M12 8.5a1.5 1.5 0 0 1 1.5 1.5v8.5H15a1.5 1.5 0 0 1 0 3H9a1.5 1.5 0 0 1 0-3h1.5V10A1.5 1.5 0 0 1 12 8.5M7.404 3.282a1.5 1.5 0 0 1 0 2.122A6.475 6.475 0 0 0 5.5 10c0 1.795.726 3.418 1.904 4.596a1.5 1.5 0 1 1-2.122 2.122A9.475 9.475 0 0 1 2.5 10c0-2.623 1.065-5 2.782-6.718a1.5 1.5 0 0 1 2.122 0m11.314 0A9.475 9.475 0 0 1 21.5 10c0 2.623-1.065 5-2.782 6.718a1.5 1.5 0 1 1-2.122-2.122A6.475 6.475 0 0 0 18.5 10a6.475 6.475 0 0 0-1.904-4.596 1.5 1.5 0 1 1 2.122-2.122m-8.486 2.829a1.5 1.5 0 0 1 .103 2.007l-.103.114A2.488 2.488 0 0 0 9.5 10c0 .392.09.76.248 1.088l.086.16a1.5 1.5 0 1 1-2.597 1.503A5.482 5.482 0 0 1 6.5 10c0-1.518.617-2.895 1.61-3.89a1.5 1.5 0 0 1 2.122 0Zm5.657 0A5.488 5.488 0 0 1 17.5 10c0 1-.268 1.94-.737 2.751a1.5 1.5 0 0 1-2.663-1.374l.066-.128A2.48 2.48 0 0 0 14.5 10c0-.622-.225-1.188-.601-1.627l-.131-.14a1.5 1.5 0 1 1 2.121-2.122"
					/></svg
				>
			</div>
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Live server</p>
				<p class="text-xs text-muted-foreground">
					Share your workspace in a browser on this device or your local network
				</p>
			</div>
		</div>
		<Switch
			checked={settings.liveEnabled}
			onCheckedChange={(v) => void handleToggle(v)}
			disabled={busy}
		/>
	</div>

	{#if settings.liveEnabled}
		<Separator />
		<div class="flex items-center justify-between gap-4">
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
		<div class="flex items-center justify-between gap-4">
			<div class="min-w-0">
				<p class="text-[13px] font-medium">Status</p>
				{#if status}
					<button
						type="button"
						onclick={() => void copyUrl()}
						title="Copy url"
						aria-label="Copy url"
						class="mt-1 flex max-w-full cursor-pointer items-center gap-1.5 rounded-md font-mono text-xs text-muted-foreground transition-colors hover:text-foreground"
					>
						<span class="size-1.5 shrink-0 rounded-full bg-emerald-500"></span>
						<span class="truncate">{copied ? 'Copied!' : status.url}</span>
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

		{#if error}
			<p class="text-xs text-destructive">{error}</p>
		{:else}
			<p class="text-xs text-muted-foreground">
				Your data stays on this device. While the server is on, any device on your local network can
				open the address above. You can also toggle it from the terminal with
				<code class="font-mono text-foreground/80">tack live on</code> or
				<code class="font-mono text-foreground/80">tack live off</code>
			</p>
		{/if}
	{/if}
{/if}
