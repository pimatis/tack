<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import {
		exportAll,
		exportCsv,
		exportMarkdown,
		importCsv,
		resetDatabase,
		importData,
		type ExportData
	} from '$lib/repositories/data.repository';
	import * as Select from '$lib/components/ui/select/index.js';
	import { save, open } from '@tauri-apps/plugin-dialog';
	import { invoke } from '@tauri-apps/api/core';
	import { isTauri } from '$lib/db/client';
	import { downloadText } from '$lib/utils';

	let resetOpen = $state(false);
	let resetConfirmText = $state('');
	let importing = $state(false);
	let exportLoading = $state(false);
	let exportFormat = $state('json');

	async function handleExport() {
		exportLoading = true;
		try {
			const date = new Date().toISOString().split('T')[0];
			let fileName: string;
			let content: string;
			let mime: string;
			if (exportFormat === 'csv') {
				fileName = `tack-export-${date}.csv`;
				content = await exportCsv();
				mime = 'text/csv';
			} else if (exportFormat === 'markdown') {
				fileName = `tack-export-${date}.md`;
				content = await exportMarkdown();
				mime = 'text/markdown';
			} else {
				fileName = `tack-export-${date}.json`;
				content = JSON.stringify(await exportAll(), null, 2);
				mime = 'application/json';
			}
			if (!isTauri()) {
				downloadText(fileName, content, mime);
				exportLoading = false;
				return;
			}
			const filePath = await save({
				defaultPath: fileName,
				filters: [
					{
						name: exportFormat.toUpperCase(),
						extensions: [exportFormat === 'markdown' ? 'md' : exportFormat]
					}
				]
			});
			if (!filePath) {
				exportLoading = false;
				return;
			}
			await invoke('write_file', { path: filePath, content });
		} catch (e) {
			console.error('export failed', e);
		}
		exportLoading = false;
	}

	// browser has no native open dialog: pick the file with a plain input
	function pickJsonFile(): Promise<File | null> {
		return new Promise((resolve) => {
			const input = document.createElement('input');
			input.type = 'file';
			input.accept = 'application/json,.json,text/csv,.csv';
			input.style.display = 'none';
			input.onchange = () => {
				resolve(input.files?.[0] ?? null);
				input.remove();
			};
			input.oncancel = () => {
				resolve(null);
				input.remove();
			};
			document.body.appendChild(input);
			input.click();
		});
	}

	async function handleImport() {
		importing = true;
		try {
			let text: string;
			if (isTauri()) {
				const filePath = await open({
					filters: [
						{ name: 'Data files', extensions: ['json', 'csv'] },
						{ name: 'All files', extensions: ['*'] }
					],
					multiple: false
				});
				if (!filePath) {
					importing = false;
					return;
				}
				text = await invoke<string>('read_file', { path: filePath });
			} else {
				const file = await pickJsonFile();
				if (!file) {
					importing = false;
					return;
				}
				text = await file.text();
			}
			// csv imports create fresh tasks and skip destructive reset; json is a
			// full replace as before
			if (text.trimStart().startsWith('{')) {
				const data = JSON.parse(text) as ExportData;
				await importData(data);
			} else {
				await importCsv(text);
			}
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
		<p class="text-xs text-muted-foreground">Save all data as JSON, or tasks as CSV/Markdown</p>
	</div>
	<div class="flex shrink-0 items-center gap-1.5">
		<Select.Root type="single" bind:value={exportFormat}>
			<Select.Trigger size="sm" class="w-28 capitalize">
				{exportFormat}
			</Select.Trigger>
			<Select.Content>
				<Select.Item value="json" label="JSON" />
				<Select.Item value="csv" label="CSV" />
				<Select.Item value="markdown" label="Markdown" />
			</Select.Content>
		</Select.Root>
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
