<script lang="ts">
	import {
		create as createLabel,
		update as updateLabel,
		remove as removeLabel
	} from '$lib/repositories/label.repository';
	import type { Label, LabelColor } from '$lib/types/label';
	import { labelColorMap, labelColorOptions } from '$lib/types/label';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import Icon from '$lib/components/Icon.svelte';

	type Props = {
		selectedIds?: string[];
		labels: Label[];
		onCreated?: (label: Label) => void;
		onUpdated?: (label: Label) => void;
		onRemoved?: (id: string) => void;
	};

	let { selectedIds = $bindable([]), labels, onCreated, onUpdated, onRemoved }: Props = $props();

	let newLabelName = $state('');
	let newLabelColor = $state<LabelColor>('gray');
	let creating = $state(false);
	let open = $state(false);
	let editingId = $state<string | null>(null);
	let editName = $state('');
	let editColor = $state<LabelColor>('gray');

	let selectedSet = $derived(new Set(selectedIds));
	let selectedLabels = $derived(labels.filter((l) => selectedSet.has(l.id)));

	function toggle(id: string) {
		if (selectedSet.has(id)) {
			selectedIds = selectedIds.filter((l) => l !== id);
		} else {
			selectedIds = [...selectedIds, id];
		}
	}

	async function handleCreate() {
		const name = newLabelName.trim();
		if (!name) return;

		creating = true;
		try {
			const label = await createLabel({ name, color: newLabelColor });
			onCreated?.(label);
			selectedIds = [...selectedIds, label.id];
			newLabelName = '';
			newLabelColor = 'gray';
		} catch {
			// ignore
		} finally {
			creating = false;
		}
	}

	function startEdit(label: Label) {
		editingId = label.id;
		editName = label.name;
		editColor = label.color;
	}

	function cancelEdit() {
		editingId = null;
	}

	async function handleSaveEdit() {
		if (!editingId || !editName.trim()) return;
		const id = editingId;
		try {
			const updated = await updateLabel(id, { name: editName.trim(), color: editColor });
			if (updated) onUpdated?.(updated);
			editingId = null;
		} catch {
			// ignore
		}
	}

	async function handleDelete(label: Label) {
		try {
			await removeLabel(label.id);
			selectedIds = selectedIds.filter((id) => id !== label.id);
			onRemoved?.(label.id);
		} catch {
			// ignore
		}
	}
</script>

<Popover.Root bind:open>
	<Popover.Trigger>
		{#snippet child({ props })}
			<Button
				{...props}
				variant="outline"
				size="sm"
				class="flex h-8 w-full items-center justify-start gap-1.5 rounded-lg border-border bg-muted/30 px-2.5 text-[12px] font-normal shadow-none transition-colors hover:bg-muted/50"
			>
				{#if selectedLabels.length > 0}
					<div class="flex flex-wrap items-center gap-1">
						{#each selectedLabels.slice(0, 3) as label (label.id)}
							<span
								class="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium {labelColorMap[
									label.color
								].badge}"
							>
								<span class="size-1.5 rounded-full {labelColorMap[label.color].dot}"></span>
								{label.name}
							</span>
						{/each}
						{#if selectedLabels.length > 3}
							<span class="text-[11px] text-muted-foreground">+{selectedLabels.length - 3}</span>
						{/if}
					</div>
				{:else}
					<Icon icon="tag" size={13} class="text-muted-foreground" />
					<span>Labels</span>
				{/if}
			</Button>
		{/snippet}
	</Popover.Trigger>
	<Popover.Content class="w-[calc(100vw-2rem)] max-w-60 p-1.5" align="start">
		<!-- label list -->
		{#if labels.length > 0}
			<div class="flex flex-col gap-px">
				{#each labels as label (label.id)}
					{#if editingId === label.id}
						<!-- edit mode -->
						<div class="flex flex-col gap-1.5 rounded-md bg-muted/40 px-2 py-1.5">
							<div class="flex items-center gap-2">
								<span class="size-2.5 shrink-0 rounded-full {labelColorMap[editColor].dot}"></span>
								<Input
									bind:value={editName}
									onkeydown={(e) => {
										if (e.key === 'Enter') {
											e.preventDefault();
											void handleSaveEdit();
										}
										if (e.key === 'Escape') cancelEdit();
									}}
									class="h-6 w-full text-[13px]"
									aria-label="Edit label name"
								/>
							</div>
							<div class="flex items-center gap-1">
								{#each labelColorOptions as color (color)}
									<button
										type="button"
										onclick={() => (editColor = color)}
										class="size-3.5 rounded-full {labelColorMap[color]
											.dot} transition-all {editColor === color
											? 'scale-110'
											: 'opacity-50 hover:opacity-100'}"
										aria-label={`Color ${color}`}
									></button>
								{/each}
								<div class="flex-1"></div>
								<Button
									variant="ghost"
									size="xs"
									class="h-auto p-0 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
									onclick={cancelEdit}>Cancel</Button
								>
								<Button
									variant="ghost"
									size="xs"
									class="h-auto p-0 text-[11px] font-medium text-foreground transition-colors hover:text-primary"
									onclick={() => void handleSaveEdit()}>Save</Button
								>
							</div>
						</div>
					{:else}
						<!-- view mode -->
						<div
							class="group/label flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
						>
							<span class="size-2.5 shrink-0 rounded-full {labelColorMap[label.color].dot}"></span>
							<Button
								variant="ghost"
								class="h-auto min-w-0 flex-1 truncate p-0 text-left text-[13px] hover:bg-transparent"
								onclick={() => toggle(label.id)}
							>
								{label.name}
							</Button>
							{#if selectedSet.has(label.id)}
								<Icon icon="check" size={14} class="shrink-0 text-muted-foreground" />
							{/if}
							<!-- edit + delete buttons (visible on touch, hover on desktop) -->
							<div
								class="flex shrink-0 items-center gap-0.5 opacity-100 sm:opacity-0 sm:transition-opacity sm:group-hover/label:opacity-100"
							>
								<Tooltip.Root>
									<Tooltip.Trigger>
										{#snippet child({ props })}
											<Button
												{...props}
												variant="ghost"
												size="icon-xs"
												class="flex size-4 items-center justify-center rounded text-muted-foreground/50 transition-colors hover:text-foreground"
												onclick={(e) => {
													e.stopPropagation();
													startEdit(label);
												}}
												aria-label="Edit label"
											>
												<Icon icon="pencil" size={11} />
											</Button>
										{/snippet}
									</Tooltip.Trigger>
									<Tooltip.Content side="top">Edit</Tooltip.Content>
								</Tooltip.Root>
								<Tooltip.Root>
									<Tooltip.Trigger>
										{#snippet child({ props })}
											<Button
												{...props}
												variant="ghost"
												size="icon-xs"
												class="flex size-4 items-center justify-center rounded text-muted-foreground/50 transition-colors hover:text-destructive"
												onclick={(e) => {
													e.stopPropagation();
													void handleDelete(label);
												}}
												aria-label="Delete label"
											>
												<Icon icon="trash" size={11} />
											</Button>
										{/snippet}
									</Tooltip.Trigger>
									<Tooltip.Content side="top">Delete</Tooltip.Content>
								</Tooltip.Root>
							</div>
						</div>
					{/if}
				{/each}
			</div>
		{:else}
			<div class="px-2 py-3 text-[12px] text-muted-foreground/50">No labels yet</div>
		{/if}

		<!-- create new label -->
		<div class="mt-1.5 border-t border-border/60 pt-1.5">
			<div
				class="flex items-center gap-2 rounded-md bg-muted/30 px-2 py-2 transition-colors focus-within:bg-muted/50"
			>
				<span class="size-2.5 shrink-0 rounded-full {labelColorMap[newLabelColor].dot}"></span>
				<Input
					bind:value={newLabelName}
					placeholder="Create new label"
					onkeydown={(e) => {
						if (e.key === 'Enter') {
							e.preventDefault();
							void handleCreate();
						}
					}}
					class="h-6 w-full text-[13px] placeholder:text-muted-foreground/40"
				/>
				{#if newLabelName.trim()}
					<Button
						variant="ghost"
						size="icon-xs"
						class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground transition-colors hover:text-foreground"
						onclick={() => void handleCreate()}
						disabled={creating}
						aria-label="Add label"
					>
						<Icon icon="add" size={14} />
					</Button>
				{/if}
			</div>
			<div class="flex items-center gap-1 px-1.5 pt-1.5">
				{#each labelColorOptions as color (color)}
					<button
						type="button"
						onclick={() => (newLabelColor = color)}
						class="size-3.5 rounded-full {labelColorMap[color]
							.dot} transition-all {newLabelColor === color
							? 'scale-110'
							: 'opacity-50 hover:opacity-100'}"
						aria-label={`Color ${color}`}
					></button>
				{/each}
			</div>
		</div>
	</Popover.Content>
</Popover.Root>
