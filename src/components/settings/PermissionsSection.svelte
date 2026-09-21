<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { isTauri } from '$lib/db/client';
	import {
		notificationPermission,
		requestNotificationPermission,
		openNotificationSettings,
		type PermissionState
	} from '$lib/permissions/permissions.service';

	const browser = !isTauri();

	let status = $state<PermissionState | 'checking'>('checking');
	let busy = $state(false);
	let hint = $state('');

	async function refresh() {
		status = await notificationPermission();
	}

	// ask the os first; macOS only prompts once, so a previous denial just
	// re-opens the notification pane in system settings
	async function grant() {
		busy = true;
		hint = '';
		try {
			const granted = await requestNotificationPermission();
			status = granted ? 'granted' : 'prompt';
			if (!granted) {
				await openNotificationSettings();
				hint = 'macOS only asks once. Turn notifications on for Tack in the list above.';
			}
		} catch {
			hint = 'Could not open System Settings.';
		}
		busy = false;
	}

	async function openSettings() {
		hint = '';
		try {
			await openNotificationSettings();
		} catch {
			hint = 'Could not open System Settings.';
		}
	}

	onMount(() => {
		if (browser) {
			status = 'unsupported';
			return;
		}
		void refresh();
	});
</script>

{#if browser}
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Permissions</p>
			<p class="text-xs text-muted-foreground">
				System permissions are managed in the desktop app
			</p>
		</div>
	</div>
{:else}
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-[13px] font-medium">Notifications</p>
			<p class="text-xs text-muted-foreground">
				Show a system alert when a task reminder is due
			</p>
		</div>
		<div class="flex shrink-0 items-center gap-2">
			{#if status === 'checking'}
				<span class="text-xs text-muted-foreground">Checking…</span>
			{:else if status === 'granted'}
				<span
					class="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-500"
				>
					<span class="size-1.5 rounded-full bg-emerald-500"></span>
					Granted
				</span>
			{:else if status === 'unsupported'}
				<span class="text-xs text-muted-foreground">Installed app only</span>
			{:else if status === 'denied'}
				<Button variant="outline" size="sm" onclick={() => void openSettings()}>
					Open Settings
				</Button>
			{:else}
				<Button variant="outline" size="sm" onclick={() => void openSettings()}>
					Open Settings
				</Button>
				<Button size="sm" onclick={() => void grant()} disabled={busy}>
					{busy ? 'Requesting…' : 'Grant'}
				</Button>
			{/if}
		</div>
	</div>

	{#if hint}
		<p class="text-xs text-amber-500">{hint}</p>
	{/if}

	<Separator class="bg-border/40" />

	<p class="text-xs text-muted-foreground">
		{#if status === 'unsupported'}
			Notifications need the installed app, not a dev build.
		{:else}
			Tack only asks for notifications, to fire task reminders. If no prompt appears the setting
			was already decided, so open System Settings to change it.
		{/if}
	</p>
{/if}
