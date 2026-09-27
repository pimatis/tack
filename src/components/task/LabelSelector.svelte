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
					<svg class="text-muted-foreground" width="13" height="13" viewBox="0 0 24 24" fill="none"
						><path
							fill="currentColor"
							d="M10.537 2.164a3 3 0 0 1 2.244.727l.15.14 7.822 7.823a3 3 0 0 1 .135 4.098l-.135.144-5.657 5.657a3 3 0 0 1-4.098.135l-.144-.135L3.03 12.93a3 3 0 0 1-.878-2.188l.011-.205.472-5.185a3 3 0 0 1 2.537-2.695l.179-.021zM8.024 8.025a2 2 0 1 0 2.829 2.829 2 2 0 0 0-2.829-2.829"
						/></svg
					>
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
									class="h-6 w-full border-none bg-transparent px-2 text-[13px] text-foreground outline-none"
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
								<svg
									class="shrink-0 text-muted-foreground"
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									><path
										fill="currentColor"
										d="M13.06 16.06a1.5 1.5 0 0 1-2.12 0l-5.658-5.656a1.5 1.5 0 1 1 2.122-2.121L12 12.879l4.596-4.596a1.5 1.5 0 0 1 2.122 2.12l-5.657 5.658Z"
									/></svg
								>
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
												<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
													><path
														fill="currentColor"
														d="M20.131 3.16a3 3 0 0 0-4.242 0l-.707.708 4.95 4.95.706-.707a3 3 0 0 0 0-4.243l-.707-.707Zm-1.414 7.072-4.95-4.95-9.09 9.091a1.5 1.5 0 0 0-.401.724l-1.029 4.455a1 1 0 0 0 1.2 1.2l4.456-1.028a1.5 1.5 0 0 0 .723-.401z"
													/></svg
												>
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
												<svg width="11" height="11" viewBox="0 0 24 24" fill="none"
													><path
														fill="currentColor"
														d="M14.28 2a2 2 0 0 1 1.897 1.368L16.72 5H20a1 1 0 1 1 0 2l-.003.071-.867 12.143A3 3 0 0 1 16.138 22H7.862a3 3 0 0 1-2.992-2.786L4.003 7.07A1.01 1.01 0 0 1 4 7a1 1 0 0 1 0-2h3.28l.543-1.632A2 2 0 0 1 9.721 2zM9 10a1 1 0 0 0-.993.883L8 11v6a1 1 0 0 0 1.993.117L10 17v-6a1 1 0 0 0-1-1m6 0a1 1 0 0 0-1 1v6a1 1 0 1 0 2 0v-6a1 1 0 0 0-1-1m-.72-6H9.72l-.333 1h5.226z"
													/></svg
												>
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
					class="h-6 w-full border-none bg-transparent px-2 text-[13px] text-foreground outline-none placeholder:text-muted-foreground/40"
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
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none"
							><path
								fill="currentColor"
								d="M10.5 20a1.5 1.5 0 0 0 3 0v-6.5H20a1.5 1.5 0 0 0 0-3h-6.5V4a1.5 1.5 0 0 0-3 0v6.5H4a1.5 1.5 0 0 0 0 3h6.5z"
							/></svg
						>
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
