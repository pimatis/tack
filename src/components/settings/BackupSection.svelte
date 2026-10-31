<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Switch } from '$lib/components/ui/switch/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Spinner } from '$lib/components/ui/spinner/index.js';
	import * as Collapsible from '$lib/components/ui/collapsible/index.js';
	import { getSettings, setSettings } from '$lib/stores/settings';
	import Icon from '$lib/components/Icon.svelte';
	import {
		listBackups,
		createBackup,
		restoreBackup,
		deleteBackup,
		deleteAllBackups,
		type BackupInfo
	} from '$lib/backup/backup.service';

	let backups = $state<BackupInfo[]>([]);
	let loading = $state(true);
	let creating = $state(false);
	let error = $state<string | null>(null);
	let restoreTarget = $state<BackupInfo | null>(null);
	let restoring = $state(false);
	let deleteTarget = $state<BackupInfo | null>(null);
	let deleting = $state(false);
	let deleteAllOpen = $state(false);
	let deletingAll = $state(false);
	let settings = $state(getSettings());

	const intervalOptions = [
		{ value: '6', label: 'Every 6 hours' },
		{ value: '12', label: 'Every 12 hours' },
		{ value: '24', label: 'Every 24 hours' },
		{ value: '168', label: 'Every 7 days' }
	];

	function updateSetting<K extends keyof typeof settings>(key: K, value: (typeof settings)[K]) {
		settings = setSettings({ [key]: value });
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function formatDate(iso: string): string {
		const d = new Date(iso);
		if (Number.isNaN(d.getTime())) return iso;
		return new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}).format(d);
	}

	// full date for snapshot rows, falls back to raw name on parse failure
	function formatDateLong(iso: string): string {
		const d = new Date(iso);
		if (Number.isNaN(d.getTime())) return iso;
		return new Intl.DateTimeFormat('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}).format(d);
	}

	async function load() {
		loading = true;
		error = null;
		try {
			backups = await listBackups();
		} catch (e) {
			console.error('failed to list backups', e);
			error = 'Failed to load backups';
		}
		loading = false;
	}

	async function handleCreate() {
		creating = true;
		error = null;
		try {
			await createBackup(settings.backupKeepCount);
			await load();
		} catch (e) {
			console.error('backup failed', e);
			error = 'Backup failed';
		}
		creating = false;
	}

	async function handleRestore() {
		if (!restoreTarget) return;
		restoring = true;
		error = null;
		try {
			await restoreBackup(restoreTarget.name);
			window.location.reload();
		} catch (e) {
			console.error('restore failed', e);
			error = 'Restore failed';
			restoring = false;
			restoreTarget = null;
		}
	}

	async function handleDelete() {
		if (!deleteTarget) return;
		deleting = true;
		error = null;
		try {
			await deleteBackup(deleteTarget.name);
			await load();
		} catch (e) {
			console.error('delete failed', e);
			error = 'Failed to delete snapshot';
		}
		deleting = false;
		deleteTarget = null;
	}

	async function handleDeleteAll() {
		deletingAll = true;
		error = null;
		try {
			await deleteAllBackups();
			await load();
			deleteAllOpen = false;
		} catch (e) {
			console.error('delete all failed', e);
			error = 'Failed to delete snapshots';
		}
		deletingAll = false;
	}

	onMount(() => {
		void load();

		// keep the snapshot list in sync with CLI backup/restore/delete
		let refreshTimer: ReturnType<typeof setTimeout> | null = null;
		let unlisten: () => void = () => {};
		void import('@tauri-apps/api/event')
			.then(({ listen }) =>
				listen('backups-changed', () => {
					if (refreshTimer) clearTimeout(refreshTimer);
					refreshTimer = setTimeout(() => void load(), 200);
				})
			)
			.then((fn) => (unlisten = fn))
			.catch(() => {});

		return () => {
			unlisten();
			if (refreshTimer) clearTimeout(refreshTimer);
		};
	});
</script>

<div class="flex flex-wrap items-center justify-between gap-2">
	<div>
		<p class="text-[13px] font-medium">Local backups</p>
		<p class="text-xs text-muted-foreground">
			Automatic snapshots of the database and attachments, stored in tack-data/backups
		</p>
	</div>
	<Switch
		checked={settings.backupEnabled}
		onCheckedChange={(v) => updateSetting('backupEnabled', v)}
	/>
</div>

{#if settings.backupEnabled}
	<Separator />

	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<p class="text-[13px] font-medium">Backup schedule</p>
			<p class="text-xs text-muted-foreground">How often a new snapshot is taken</p>
		</div>
		<Select.Root
			type="single"
			value={String(settings.backupIntervalHours)}
			onValueChange={(v) => updateSetting('backupIntervalHours', Number(v))}
		>
			<Select.Trigger class="w-40">
				{intervalOptions.find((o) => o.value === String(settings.backupIntervalHours))?.label ??
					'Every 24 hours'}
			</Select.Trigger>
			<Select.Content>
				{#each intervalOptions as opt (opt.value)}
					<Select.Item value={opt.value} label={opt.label}>{opt.label}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</div>

	<Separator />

	<div class="flex flex-wrap items-center justify-between gap-2">
		<div>
			<p class="text-[13px] font-medium">Backups to keep</p>
			<p class="text-xs text-muted-foreground">Older snapshots are removed automatically</p>
		</div>
		<div class="flex items-center gap-2">
			<Input
				type="number"
				min="1"
				max="50"
				value={settings.backupKeepCount}
				oninput={(e) =>
					updateSetting(
						'backupKeepCount',
						Math.max(1, Math.min(50, Number((e.target as HTMLInputElement).value) || 1))
					)}
				class="w-16 text-center"
			/>
			<span class="text-xs text-muted-foreground">backups</span>
		</div>
	</div>
{/if}

<Separator />

<!-- snapshots accordion -->
<div class="rounded-lg border border-border bg-muted/30">
	<Collapsible.Root open class="group/snapshots">
		<div
			class="flex items-center gap-1.5 rounded-t-lg px-3 py-2.5 text-[13px] font-medium transition-colors hover:bg-muted/60"
		>
			<Collapsible.Trigger
				class="flex min-w-0 flex-1 cursor-pointer list-none items-center gap-1.5 text-left text-[13px] font-medium"
			>
				<Icon
					icon="arrow-right"
					size={14}
					class="shrink-0 text-muted-foreground transition-transform duration-150 group-data-[state=open]/snapshots:rotate-90"
				/>
				<span>Snapshots</span>
				{#if !loading && backups.length > 0}
					<span class="text-xs font-normal text-muted-foreground">{backups.length}</span>
				{/if}
				<span class="text-xs font-normal text-muted-foreground">
					{#if loading}
						· loading
					{:else if backups.length > 0}
						· last one {formatDate(backups[0].createdAt)}
					{:else}
						· none yet
					{/if}
				</span>
			</Collapsible.Trigger>
			<span class="ml-auto flex items-center gap-1">
				{#if !loading && backups.length > 0}
					<Button
						variant="ghost"
						size="sm"
						class="h-7 px-2 text-xs text-muted-foreground hover:text-destructive"
						onclick={(e) => {
							e.stopPropagation();
							deleteAllOpen = true;
						}}
					>
						Delete all
					</Button>
				{/if}
				<Button
					variant="ghost"
					size="sm"
					class="h-7 px-2 text-xs"
					onclick={(e) => {
						e.stopPropagation();
						void handleCreate();
					}}
					disabled={creating}
				>
					{#if creating}
						<Spinner class="size-3" />
						Backing up...
					{:else}
						Back up now
					{/if}
				</Button>
			</span>
		</div>

		<Collapsible.Content>
			{#if error}
				<p class="px-3 pb-2 text-xs text-destructive" role="alert">{error}</p>
			{/if}

			{#if loading}
				<div
					class="flex items-center gap-2 border-t border-border px-3 py-3 text-xs text-muted-foreground"
				>
					<Spinner class="size-3" />
					<span>Loading snapshots...</span>
				</div>
			{:else if backups.length > 0}
				<div class="border-t border-border">
					{#each backups as backup (backup.name)}
						<div
							class="flex items-center justify-between gap-2 border-b border-border/60 px-3 py-2 last:border-b-0"
						>
							<div class="min-w-0">
								<p class="truncate text-[12px] font-medium">{formatDateLong(backup.createdAt)}</p>
								<p class="truncate text-[11px] text-muted-foreground">
									{backup.name} · {formatSize(backup.sizeBytes)}
								</p>
							</div>
							<div class="flex shrink-0 items-center gap-0.5">
								<Button
									variant="ghost"
									size="sm"
									class="h-7 px-2 text-xs"
									onclick={() => (restoreTarget = backup)}
								>
									Restore
								</Button>
								<Button
									variant="ghost"
									size="icon-sm"
									class="text-muted-foreground hover:text-destructive"
									aria-label="Delete snapshot"
									onclick={() => (deleteTarget = backup)}
								>
									<Icon icon="trash" size={14} />
								</Button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<p class="border-t border-border px-3 py-3 text-xs text-muted-foreground">
					No snapshots yet. Create one to keep a safe copy of your data.
				</p>
			{/if}
		</Collapsible.Content>
	</Collapsible.Root>
</div>

<!-- restore dialog -->
<Dialog.Root
	open={restoreTarget !== null}
	onOpenChange={(open) => {
		if (!open) restoreTarget = null;
	}}
>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Restore snapshot</Dialog.Title>
			<Dialog.Description>
				This will replace all current data with the snapshot from
				{restoreTarget ? formatDateLong(restoreTarget.createdAt) : ''}. The app will reload
				afterwards.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button
				variant="outline"
				size="sm"
				onclick={() => (restoreTarget = null)}
				disabled={restoring}
			>
				Cancel
			</Button>
			<Button
				variant="destructive"
				size="sm"
				onclick={() => void handleRestore()}
				disabled={restoring}
			>
				{restoring ? 'Restoring...' : 'Restore'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- delete dialog -->
<Dialog.Root
	open={deleteTarget !== null}
	onOpenChange={(open) => {
		if (!open) deleteTarget = null;
	}}
>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Delete snapshot</Dialog.Title>
			<Dialog.Description>
				This will permanently remove the snapshot from
				{deleteTarget ? formatDateLong(deleteTarget.createdAt) : ''}
				({deleteTarget ? formatSize(deleteTarget.sizeBytes) : ''}). Your current data is not
				affected.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" size="sm" onclick={() => (deleteTarget = null)} disabled={deleting}>
				Cancel
			</Button>
			<Button
				variant="destructive"
				size="sm"
				onclick={() => void handleDelete()}
				disabled={deleting}
			>
				{deleting ? 'Deleting...' : 'Delete'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- delete all dialog -->
<Dialog.Root
	open={deleteAllOpen}
	onOpenChange={(open) => {
		if (!open && !deletingAll) deleteAllOpen = false;
	}}
>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Delete all snapshots</Dialog.Title>
			<Dialog.Description>
				This will permanently remove all {backups.length} snapshots. Your current data is not affected.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button
				variant="outline"
				size="sm"
				onclick={() => (deleteAllOpen = false)}
				disabled={deletingAll}
			>
				Cancel
			</Button>
			<Button
				variant="destructive"
				size="sm"
				onclick={() => void handleDeleteAll()}
				disabled={deletingAll}
			>
				{deletingAll ? 'Deleting...' : 'Delete all'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
