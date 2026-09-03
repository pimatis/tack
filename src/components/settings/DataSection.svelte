<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import {
		exportAll,
		resetDatabase,
		importData,
		type ExportData
	} from '$lib/repositories/data.repository';
	import { save, open } from '@tauri-apps/plugin-dialog';
	import { invoke } from '@tauri-apps/api/core';

	let resetOpen = $state(false);
	let resetConfirmText = $state('');
	let importing = $state(false);
	let exportLoading = $state(false);

	async function handleExport() {
		exportLoading = true;
		try {
			const filePath = await save({
				defaultPath: `tack-export-${new Date().toISOString().split('T')[0]}.json`,
				filters: [{ name: 'JSON', extensions: ['json'] }]
			});
			if (!filePath) {
				exportLoading = false;
				return;
			}
			const data = await exportAll();
			await invoke('write_file', { path: filePath, content: JSON.stringify(data, null, 2) });
		} catch (e) {
			console.error('export failed', e);
		}
		exportLoading = false;
	}

	async function handleImport() {
		importing = true;
		try {
			const filePath = await open({
				filters: [{ name: 'JSON', extensions: ['json'] }],
				multiple: false
			});
			if (!filePath) {
				importing = false;
				return;
			}
			const text = await invoke<string>('read_file', { path: filePath });
			const data = JSON.parse(text) as ExportData;
			await importData(data);
			window.location.reload();
		} catch (e) {
			console.error('import failed', e);
		}
		importing = false;
	}

	async function handleReset() {
		if (resetConfirmText !== 'Delete') return;
		await resetDatabase();
		resetOpen = false;
		resetConfirmText = '';
		window.location.reload();
	}
</script>

<!-- export -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div class="min-w-0">
		<p class="text-[13px] font-medium">Export data</p>
		<p class="text-xs text-muted-foreground">Save all projects, tasks and labels as a JSON file</p>
	</div>
	<Button
		variant="outline"
		size="sm"
		onclick={() => void handleExport()}
		disabled={exportLoading}
		class="shrink-0"
	>
		{exportLoading ? 'Exporting...' : 'Export'}
	</Button>
</div>

<Separator />

<!-- import -->
<div class="flex flex-wrap items-center justify-between gap-2">
	<div class="min-w-0">
		<p class="text-[13px] font-medium">Import data</p>
		<p class="text-xs text-muted-foreground">
			Restore from a previously exported JSON file. This will replace all current data.
		</p>
	</div>
	<Button
		variant="outline"
		size="sm"
		onclick={() => void handleImport()}
		disabled={importing}
		class="shrink-0"
	>
		{importing ? 'Importing...' : 'Import'}
	</Button>
</div>

<Separator />

<!-- reset -->
<div class="rounded-lg border border-destructive/30 bg-destructive/5 p-4">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div class="min-w-0">
			<p class="text-[13px] font-medium text-destructive">Delete all data</p>
			<p class="text-xs text-muted-foreground">
				Permanently delete all projects, tasks, labels and attachments. This cannot be undone.
			</p>
		</div>
		<Dialog.Root bind:open={resetOpen}>
			<Dialog.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="destructive" size="sm">Delete</Button>
				{/snippet}
			</Dialog.Trigger>
			<Dialog.Content>
				<Dialog.Header>
					<Dialog.Title>Delete all data</Dialog.Title>
					<Dialog.Description>
						This will permanently delete all projects, tasks, labels and attachments. Type "Delete"
						to confirm.
					</Dialog.Description>
				</Dialog.Header>
				<Input type="text" bind:value={resetConfirmText} placeholder="Delete" class="w-full" />
				<Dialog.Footer>
					<Dialog.Close>
						{#snippet child({ props })}
							<Button {...props} variant="outline" size="sm">Cancel</Button>
						{/snippet}
					</Dialog.Close>
					<Button
						variant="destructive"
						size="sm"
						disabled={resetConfirmText !== 'Delete'}
						onclick={() => void handleReset()}
					>
						Delete everything
					</Button>
				</Dialog.Footer>
			</Dialog.Content>
		</Dialog.Root>
	</div>
</div>
