<script lang="ts">
	import { onMount } from 'svelte';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { isTauri } from '$lib/db/client';
	import { getAgentStatus, type AgentStatus } from '$lib/agent/agent.service';
	import {
		findRecentAgentActivity,
		type AgentActivityEntry
	} from '$lib/repositories/activity.repository';
	import type { Settings } from '$lib/types/settings';
	import Icon from '$lib/components/Icon.svelte';

	let {
		settings,
		update
	}: {
		settings: Settings;
		update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
	} = $props();

	const browser = !isTauri();

	let status = $state<AgentStatus | null>(null);
	let error = $state('');
	let busy = $state(false);
	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;
	let activity = $state<AgentActivityEntry[]>([]);

	async function refresh() {
		status = await getAgentStatus();
		try {
			activity = await findRecentAgentActivity();
		} catch {
			activity = [];
		}
	}

	function actionText(entry: AgentActivityEntry): string {
		return entry.action.replace(/_/g, ' ');
	}

	function relativeTime(value: string): string {
		const diffMin = Math.round((Date.now() - new Date(value).getTime()) / 60000);
		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;
		const hours = Math.round(diffMin / 60);
		if (hours < 24) return `${hours}h ago`;
		return `${Math.round(hours / 24)}d ago`;
	}

	async function handleToggle(enabled: boolean) {
		busy = true;
		error = '';
		try {
			update('agentEnabled', enabled);
			// give the agent manager a moment to start or stop the server
			await new Promise((resolve) => setTimeout(resolve, 400));
			await refresh();
		} catch {
			error = 'the agent server could not be reached';
		}
		busy = false;
	}

	async function handlePortChange(value: string) {
		const port = Number(value);
		if (!Number.isInteger(port) || port < 1024 || port > 65535) return;
		error = '';
		update('agentPort', port);
		// the manager restarts the server on the new port
		await new Promise((resolve) => setTimeout(resolve, 400));
		await refresh();
	}

	async function copyUrl() {
		if (!status) return;
		try {
			await navigator.clipboard.writeText(`${status.url}/api/agent/help`);
		} catch {
			return;
		}
		copied = true;
		if (copyTimer) clearTimeout(copyTimer);
		copyTimer = setTimeout(() => (copied = false), 3000);
	}

	onMount(() => {
		void refresh();
		const interval = window.setInterval(() => void refresh(), 3000);
		const onStatus = () => void refresh();
		const onError = (event: Event) => {
			error = (event as CustomEvent<string>).detail;
		};
		window.addEventListener('agent-status-changed', onStatus);
		window.addEventListener('agent-error-changed', onError);
		return () => {
			window.clearInterval(interval);
			window.removeEventListener('agent-status-changed', onStatus);
			window.removeEventListener('agent-error-changed', onError);
			if (copyTimer) clearTimeout(copyTimer);
		};
	});
</script>

<div class="flex flex-wrap items-center justify-between gap-3">
	<div class="min-w-0">
		<p class="text-[13px] font-medium">Agent control</p>
		<p class="text-xs text-muted-foreground">
			{#if browser}
				UI control for AI agents only runs in the desktop app.
			{:else}
				Let AI agents see the app, click and type on your behalf
			{/if}
		</p>
	</div>
	{#if !browser}
		<Switch
			checked={settings.agentEnabled}
			onCheckedChange={(v) => void handleToggle(v)}
			disabled={busy}
		/>
	{/if}
</div>

{#if !browser && settings.agentEnabled}
	<Separator />
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<p class="text-[13px] font-medium">Port</p>
			<p class="text-xs text-muted-foreground">Loopback address agents connect to</p>
		</div>
		<Input
			type="number"
			min={1024}
			max={65535}
			value={settings.agentPort}
			oninput={(e) => void handlePortChange((e.currentTarget as HTMLInputElement).value)}
			class="w-28 text-right"
		/>
	</div>

	<Separator />
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Status</p>
			{#if status}
				<button
					type="button"
					onclick={() => void copyUrl()}
					title="Copy api docs url"
					aria-label="Copy api docs url"
					class="mt-1 flex max-w-full cursor-pointer items-center gap-1.5 rounded-md font-mono text-xs text-muted-foreground transition-colors hover:text-foreground"
				>
					<span class="size-1.5 shrink-0 rounded-full bg-emerald-500"></span>
					<span class="truncate">{copied ? 'Copied!' : `${status.url}/api/agent/help`}</span>
				</button>
			{:else}
				<p class="text-xs text-muted-foreground">
					{error ? 'could not start' : 'starting…'}
				</p>
			{/if}
		</div>
		<span
			class="inline-flex shrink-0 items-center gap-1.5 text-xs font-medium {status
				? 'text-emerald-600 dark:text-emerald-500'
				: 'text-muted-foreground'}"
		>
			{#if status}
				<Icon icon="check-circle" size={15} />
				Listening
			{/if}
		</span>
	</div>

	<Separator />
	<div class="flex flex-col gap-1.5">
		<p class="text-[13px] font-medium">Safety</p>
		<p class="text-xs text-muted-foreground">
			The server only listens on this device, never on your network. When an agent tries a
			destructive action (delete, trash, reset), a dialog asks you to allow or deny it first.
		</p>
	</div>

	<Separator />
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Recent agent activity</p>
			<p class="text-xs text-muted-foreground">
				{activity.length === 0
					? 'Nothing agents have changed through the api yet'
					: 'Latest changes agents made through the api'}
			</p>
		</div>
		{#if activity.length > 0}
			<div class="flex max-w-[60%] flex-col items-end gap-0.5">
				{#each activity as entry (entry.id)}
					<span class="font-mono text-[11px] text-muted-foreground" title={entry.createdAt}>
						{entry.displayNumber}
						{actionText(entry)} · {relativeTime(entry.createdAt)}
					</span>
				{/each}
			</div>
		{/if}
	</div>

	{#if error}
		<p class="text-xs text-destructive">{error}</p>
	{:else}
		<p class="text-xs text-muted-foreground">
			Agents drive the app with
			<code class="font-mono text-foreground/80">tack ui snapshot</code>,
			<code class="font-mono text-foreground/80">tack ui click --ref 3</code> and
			<code class="font-mono text-foreground/80">tack ui screenshot</code>, or connect an
			MCP-capable agent with <code class="font-mono text-foreground/80">tack mcp</code>.
		</p>
	{/if}
{/if}
