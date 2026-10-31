<script lang="ts">
	import { update } from '$lib/repositories/project.repository';
	import type { Project } from '$lib/types/project';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import MarkdownRenderer from './MarkdownRenderer.svelte';
	import { normalizePrefix, isValidPrefix } from '$lib/prefix';
	import Icon from '$lib/components/Icon.svelte';

	type Props = { open?: boolean; project: Project | null; onUpdated?: (project: Project) => void };
	let { open = $bindable(false), project, onUpdated }: Props = $props();
	let name = $state('');
	let prefix = $state('');
	let description = $state('');
	let previewMode = $state(false);
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let nameRef = $state<HTMLInputElement | null>(null);

	// snapshot on open; save stays disabled until something actually changes
	let original = $state<{ name: string; prefix: string; description: string } | null>(null);
	let isDirty = $derived(
		!!original &&
			(original.name !== name || original.prefix !== prefix || original.description !== description)
	);

	$effect(() => {
		if (!open || !project) return;
		name = project.name;
		prefix = project.prefix;
		description = project.description ?? '';
		previewMode = false;
		error = null;
		original = {
			name: project.name,
			prefix: project.prefix,
			description: project.description ?? ''
		};
	});

	$effect(() => {
		if (open) requestAnimationFrame(() => nameRef?.focus());
	});

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (!project || !name.trim()) {
			error = 'Project name is required';
			return;
		}
		const normalizedPrefix = normalizePrefix(prefix);
		if (!isValidPrefix(normalizedPrefix)) {
			error = 'Prefix must be 2-4 letters or numbers';
			return;
		}
		submitting = true;
		try {
			const updated = await update(project.id, {
				name: name.trim(),
				prefix: normalizedPrefix,
				description: description.trim() || null
			});
			if (updated) onUpdated?.(updated);
			open = false;
		} catch {
			error = 'Failed to update project. Prefix may already be in use';
		} finally {
			submitting = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="w-[calc(100vw-2rem)] max-w-md gap-0 p-0" showCloseButton={false}>
		<Dialog.Title class="sr-only">Edit project</Dialog.Title>
		<form onsubmit={handleSubmit} class="flex flex-col">
			<!-- header -->
			<div class="flex items-center justify-between px-4 pt-4 pb-3 sm:px-5">
				<span class="text-[13px] font-medium text-foreground">Edit project</span>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							class="text-muted-foreground hover:text-foreground"
						>
							<Icon icon="close" size={16} />
						</Button>
					{/snippet}
				</Dialog.Close>
			</div>

			<!-- body -->
			<div class="flex flex-col gap-3 px-4 pb-4 sm:px-5">
				<Input
					bind:ref={nameRef}
					bind:value={name}
					placeholder="Project name"
					class="h-auto px-3 py-2 text-[18px] font-semibold placeholder:text-muted-foreground/40"
				/>
				<div class="flex items-center gap-2">
					<span class="shrink-0 text-[12px] text-muted-foreground">Prefix</span>
					<Input
						bind:value={prefix}
						placeholder="WEB"
						maxlength={4}
						class="h-7 w-20 text-[12px] font-medium"
					/>
				</div>

				<!-- description with markdown support -->
				<div class="flex flex-col gap-1.5">
					<div class="flex items-center justify-between">
						<span class="text-[12px] text-muted-foreground">Description</span>
						{#if description.trim()}
							<Button
								variant="ghost"
								class="flex h-auto items-center gap-1 p-0 text-[11px] text-muted-foreground/60 transition-colors hover:text-foreground"
								onclick={() => (previewMode = !previewMode)}
							>
								{#if previewMode}
									<Icon icon="checklist" size={11} />
									<span>Edit</span>
								{:else}
									<Icon icon="eye" size={11} />
									<span>Preview</span>
								{/if}
							</Button>
						{/if}
					</div>
					{#if previewMode}
						<div class="min-h-[80px] rounded-lg border border-border bg-muted/20 p-3">
							{#if description.trim()}
								<MarkdownRenderer content={description} />
							{:else}
								<span class="text-[12px] text-muted-foreground/50">Nothing to preview</span>
							{/if}
						</div>
					{:else}
						<Textarea
							bind:value={description}
							placeholder="Add a description... **markdown supported**"
							rows={3}
							class="min-h-[80px] text-[13px] placeholder:text-muted-foreground/50"
						/>
					{/if}
				</div>
			</div>

			{#if error}
				<p class="px-4 text-[12px] text-destructive sm:px-5" role="alert">{error}</p>
			{/if}

			<Separator />

			<!-- footer -->
			<div class="flex flex-wrap items-center justify-end gap-2 px-4 py-3 sm:px-5">
				<Button type="button" variant="ghost" size="sm" onclick={() => (open = false)}
					>Cancel</Button
				>
				<Button type="submit" size="sm" disabled={submitting || !isDirty}>
					{submitting ? 'Saving...' : 'Save changes'}
				</Button>
			</div>
		</form>
	</Dialog.Content>
</Dialog.Root>
